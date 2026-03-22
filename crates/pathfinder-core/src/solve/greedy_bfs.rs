use std::collections::BinaryHeap;
use std::cmp::Reverse;
use super::{SolveAction, SolveStep, SteppableSolver};
use crate::maze::MazeGrid;

/// Greedy Best-First Search maze solver.
///
/// Uses a priority queue ordered solely by heuristic h (distance to the goal,
/// delegated to `MazeGrid::heuristic_distance()`). Does **not** track path cost
/// (g-score), so it is fast but does **not** guarantee the shortest path.
pub struct GreedyBfsSolver {
    /// Min-heap of (h, cell_index).
    heap: BinaryHeap<Reverse<(u32, u32)>>,
    visited: Vec<bool>,
    parent: Vec<Option<u32>>,
    start: u32,
    end: u32,
    done: bool,
    found: bool,
    solution_path: Vec<u32>,
}

impl GreedyBfsSolver {
    pub fn new(maze: &dyn MazeGrid, start: u32, end: u32) -> Self {
        let cell_count = maze.cell_count();

        let mut visited = vec![false; cell_count as usize];
        visited[start as usize] = true;

        let mut heap = BinaryHeap::new();
        let h = maze.heuristic_distance(start, end);
        heap.push(Reverse((h, start)));

        GreedyBfsSolver {
            heap,
            visited,
            parent: vec![None; cell_count as usize],
            start,
            end,
            done: false,
            found: false,
            solution_path: Vec::new(),
        }
    }

    fn reconstruct_path(&mut self) {
        self.solution_path.clear();
        let mut current = self.end;
        self.solution_path.push(current);
        while current != self.start {
            match self.parent[current as usize] {
                Some(p) => {
                    current = p;
                    self.solution_path.push(current);
                }
                None => break,
            }
        }
        self.solution_path.reverse();
    }
}

impl SteppableSolver for GreedyBfsSolver {
    fn step(&mut self, maze: &dyn MazeGrid) -> Option<SolveStep> {
        if self.done {
            return None;
        }

        let current = match self.heap.pop() {
            Some(Reverse((_, cell))) => cell,
            None => {
                self.done = true;
                return None;
            }
        };

        if current == self.end {
            self.done = true;
            self.found = true;
            self.reconstruct_path();
            return Some(SolveStep {
                cell: current,
                action: SolveAction::FoundGoal,
            });
        }

        for (neighbor, dir) in maze.neighbors(current) {
            if !maze.has_wall(current, dir) && !self.visited[neighbor as usize] {
                self.visited[neighbor as usize] = true;
                self.parent[neighbor as usize] = Some(current);
                let h = maze.heuristic_distance(neighbor, self.end);
                self.heap.push(Reverse((h, neighbor)));
            }
        }

        Some(SolveStep {
            cell: current,
            action: SolveAction::Visit,
        })
    }

    fn path(&self) -> Option<&[u32]> {
        if self.found {
            Some(&self.solution_path)
        } else {
            None
        }
    }

    fn reset(&mut self, start: u32, end: u32) {
        self.start = start;
        self.end = end;
        self.visited.fill(false);
        self.visited[start as usize] = true;
        self.parent.fill(None);
        self.heap.clear();
        // Without a maze ref we use 0 as a placeholder heuristic.
        // The entry will still be popped first since the heap is empty.
        self.heap.push(Reverse((0, start)));
        self.done = false;
        self.found = false;
        self.solution_path.clear();
    }

    fn is_done(&self) -> bool {
        self.done
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maze::RectGrid;
    use crate::generate::{DfsGenerator, SteppableGenerator};
    use crate::rng::Xoshiro256;

    fn generate_maze(width: u32, height: u32, seed: u64) -> RectGrid {
        let mut grid = RectGrid::new(width, height);
        let mut rng = Xoshiro256::new(seed);
        let mut gen = DfsGenerator::new(grid.cell_count(), 0);
        while gen.step(&mut grid, &mut rng).is_some() {}
        grid
    }

    #[test]
    fn test_greedy_bfs_finds_path() {
        let grid = generate_maze(10, 10, 42);
        let start = 0;
        let end = grid.cell_count() - 1;
        let mut solver = GreedyBfsSolver::new(&grid, start, end);

        let mut steps = 0;
        while let Some(_step) = solver.step(&grid) {
            steps += 1;
        }

        assert!(solver.is_done());
        assert!(solver.path().is_some());
        let path = solver.path().unwrap();
        assert_eq!(path[0], start);
        assert_eq!(*path.last().unwrap(), end);
        assert!(path.len() >= 2);
        assert!(steps > 0);
    }

    #[test]
    fn test_greedy_bfs_path_is_valid() {
        let grid = generate_maze(8, 8, 99);
        let start = 0;
        let end = 63;
        let mut solver = GreedyBfsSolver::new(&grid, start, end);

        while solver.step(&grid).is_some() {}

        let path = solver.path().unwrap();
        for window in path.windows(2) {
            let from = window[0];
            let to = window[1];
            let neighbors = grid.neighbors(from);
            let neighbor_entry = neighbors.iter().find(|&&(n, _)| n == to);
            assert!(
                neighbor_entry.is_some(),
                "Path cells {} and {} are not neighbors",
                from,
                to
            );
            let (_, dir) = *neighbor_entry.unwrap();
            assert!(
                !grid.has_wall(from, dir),
                "Wall exists between {} and {}",
                from,
                to
            );
        }
    }

    #[test]
    fn test_greedy_bfs_start_equals_end() {
        let grid = generate_maze(5, 5, 1);
        let mut solver = GreedyBfsSolver::new(&grid, 0, 0);

        let step = solver.step(&grid);
        assert!(step.is_some());
        assert!(matches!(step.unwrap().action, SolveAction::FoundGoal));
        assert!(solver.is_done());
        assert_eq!(solver.path().unwrap(), &[0]);
    }

    #[test]
    fn test_greedy_bfs_various_sizes() {
        for &(w, h, seed) in &[(5, 5, 10), (15, 15, 77), (20, 10, 200)] {
            let grid = generate_maze(w, h, seed);
            let start = 0;
            let end = grid.cell_count() - 1;
            let mut solver = GreedyBfsSolver::new(&grid, start, end);

            while solver.step(&grid).is_some() {}

            assert!(
                solver.path().is_some(),
                "Failed for {}x{} seed={}",
                w,
                h,
                seed
            );
        }
    }

    #[test]
    fn test_greedy_bfs_reset() {
        let grid = generate_maze(6, 6, 55);
        let mut solver = GreedyBfsSolver::new(&grid, 0, 35);

        while solver.step(&grid).is_some() {}
        assert!(solver.path().is_some());

        // Reset and solve again with different endpoints
        solver.reset(5, 30);
        assert!(!solver.is_done());
        assert!(solver.path().is_none());

        while solver.step(&grid).is_some() {}
        assert!(solver.is_done());
        assert!(solver.path().is_some());
        let path = solver.path().unwrap();
        assert_eq!(path[0], 5);
        assert_eq!(*path.last().unwrap(), 30);
    }
}
