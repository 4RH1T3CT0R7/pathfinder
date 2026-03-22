use std::cmp::Ordering;
use std::collections::BinaryHeap;

use super::{SolveAction, SolveStep, SteppableSolver};
use crate::maze::MazeGrid;

/// Dijkstra's algorithm maze solver implemented as a state machine.
///
/// Equivalent to A* with h=0 (no heuristic). Uses a priority queue ordered
/// by distance from start. Guarantees the shortest path in an unweighted
/// grid maze. In an unweighted maze, the result is identical to BFS but
/// the exploration order may differ due to priority queue tie-breaking.
pub struct DijkstraSolver {
    open_set: BinaryHeap<DijkstraEntry>,
    dist: Vec<u32>,
    came_from: Vec<Option<u32>>,
    in_closed: Vec<bool>,
    start: u32,
    end: u32,
    done: bool,
    found: bool,
    solution_path: Vec<u32>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct DijkstraEntry {
    cell: u32,
    dist: u32,
}

/// Ordering for `BinaryHeap`: lower distance = higher priority.
/// Rust's `BinaryHeap` is a max-heap, so we invert the comparison.
impl Ord for DijkstraEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .dist
            .cmp(&self.dist)
            .then_with(|| self.cell.cmp(&other.cell))
    }
}

impl PartialOrd for DijkstraEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl DijkstraSolver {
    pub fn new(cell_count: u32, start: u32, end: u32) -> Self {
        let count = cell_count as usize;
        let mut dist = vec![u32::MAX; count];
        dist[start as usize] = 0;

        let mut open_set = BinaryHeap::new();
        open_set.push(DijkstraEntry {
            cell: start,
            dist: 0,
        });

        DijkstraSolver {
            open_set,
            dist,
            came_from: vec![None; count],
            in_closed: vec![false; count],
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
            match self.came_from[current as usize] {
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

impl SteppableSolver for DijkstraSolver {
    fn step(&mut self, maze: &dyn MazeGrid) -> Option<SolveStep> {
        if self.done {
            return None;
        }

        // Skip entries that have already been closed (stale duplicates in the heap)
        let current = loop {
            match self.open_set.pop() {
                Some(entry) => {
                    if !self.in_closed[entry.cell as usize] {
                        break entry.cell;
                    }
                    // stale entry, skip
                }
                None => {
                    self.done = true;
                    return None;
                }
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

        self.in_closed[current as usize] = true;
        let current_dist = self.dist[current as usize];

        // Explore neighbors through open passages
        for (neighbor, dir) in maze.neighbors(current) {
            if !maze.has_wall(current, dir) {
                if self.in_closed[neighbor as usize] {
                    continue;
                }
                let tentative_dist = current_dist + 1;
                if tentative_dist < self.dist[neighbor as usize] {
                    self.dist[neighbor as usize] = tentative_dist;
                    self.came_from[neighbor as usize] = Some(current);
                    self.open_set.push(DijkstraEntry {
                        cell: neighbor,
                        dist: tentative_dist,
                    });
                }
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
        self.dist.fill(u32::MAX);
        self.dist[start as usize] = 0;
        self.came_from.fill(None);
        self.in_closed.fill(false);
        self.open_set.clear();
        self.open_set.push(DijkstraEntry {
            cell: start,
            dist: 0,
        });
        self.start = start;
        self.end = end;
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
    use crate::generate::{DfsGenerator, SteppableGenerator};
    use crate::maze::{Direction, RectGrid};
    use crate::rng::Xoshiro256;
    use crate::solve::BfsSolver;

    fn generate_maze(width: u32, height: u32, seed: u64) -> RectGrid {
        let mut grid = RectGrid::new(width, height);
        let mut rng = Xoshiro256::new(seed);
        let mut gen = DfsGenerator::new(grid.cell_count(), 0);
        while gen.step(&mut grid, &mut rng).is_some() {}
        grid
    }

    #[test]
    fn test_dijkstra_finds_path() {
        let grid = generate_maze(10, 10, 42);
        let start = 0;
        let end = grid.cell_count() - 1;
        let mut solver = DijkstraSolver::new(grid.cell_count(), start, end);

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
    fn test_dijkstra_path_is_valid() {
        let grid = generate_maze(8, 8, 99);
        let start = 0;
        let end = 63;
        let mut solver = DijkstraSolver::new(grid.cell_count(), start, end);

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
    fn test_dijkstra_start_equals_end() {
        let grid = generate_maze(5, 5, 1);
        let mut solver = DijkstraSolver::new(grid.cell_count(), 0, 0);

        let step = solver.step(&grid);
        assert!(step.is_some());
        assert!(matches!(step.unwrap().action, SolveAction::FoundGoal));
        assert!(solver.is_done());
        assert_eq!(solver.path().unwrap(), &[0]);
    }

    #[test]
    fn test_dijkstra_shortest_path_matches_bfs() {
        // Dijkstra must produce the same shortest-path length as BFS
        for seed in 0..20 {
            let grid = generate_maze(10, 10, seed);
            let start = 0;
            let end = grid.cell_count() - 1;

            let mut bfs = BfsSolver::new(grid.cell_count(), start, end);
            while bfs.step(&grid).is_some() {}

            let mut dijkstra = DijkstraSolver::new(grid.cell_count(), start, end);
            while dijkstra.step(&grid).is_some() {}

            let bfs_path = bfs.path().unwrap();
            let dijkstra_path = dijkstra.path().unwrap();

            assert_eq!(
                bfs_path.len(),
                dijkstra_path.len(),
                "Dijkstra path length ({}) != BFS path length ({}) for seed {}",
                dijkstra_path.len(),
                bfs_path.len(),
                seed
            );
        }
    }

    #[test]
    fn test_dijkstra_on_corridor() {
        let mut grid = RectGrid::new(3, 1);
        grid.remove_wall(0, Direction::EAST);
        grid.remove_wall(1, Direction::EAST);

        let mut solver = DijkstraSolver::new(3, 0, 2);
        while solver.step(&grid).is_some() {}

        let path = solver.path().unwrap();
        assert_eq!(path, &[0, 1, 2]);
    }

    #[test]
    fn test_dijkstra_reset_works() {
        let grid = generate_maze(6, 6, 42);
        let cell_count = grid.cell_count();
        let mut solver = DijkstraSolver::new(cell_count, 0, cell_count - 1);

        while solver.step(&grid).is_some() {}
        assert!(solver.path().is_some());

        // Reset and solve again with different endpoints
        solver.reset(5, 10);
        assert!(!solver.is_done());
        assert!(solver.path().is_none());

        while solver.step(&grid).is_some() {}
        assert!(solver.is_done());
        assert!(solver.path().is_some());
        let path = solver.path().unwrap();
        assert_eq!(path[0], 5);
        assert_eq!(*path.last().unwrap(), 10);
    }

    #[test]
    fn test_dijkstra_finds_path_various_seeds() {
        for seed in 0..20 {
            let grid = generate_maze(12, 12, seed);
            let start = 0;
            let end = grid.cell_count() - 1;
            let mut solver = DijkstraSolver::new(grid.cell_count(), start, end);

            while solver.step(&grid).is_some() {}

            assert!(
                solver.path().is_some(),
                "Dijkstra failed to find a path for seed {}",
                seed
            );
            let path = solver.path().unwrap();
            assert_eq!(path[0], start);
            assert_eq!(*path.last().unwrap(), end);
        }
    }

    #[test]
    fn test_dijkstra_optimal_across_sizes() {
        // Dijkstra must match BFS path length across various maze sizes
        for (w, h) in [(5, 5), (10, 10), (15, 15), (20, 20)] {
            let grid = generate_maze(w, h, 123);
            let start = 0;
            let end = grid.cell_count() - 1;

            let mut bfs = BfsSolver::new(grid.cell_count(), start, end);
            while bfs.step(&grid).is_some() {}

            let mut dijkstra = DijkstraSolver::new(grid.cell_count(), start, end);
            while dijkstra.step(&grid).is_some() {}

            let bfs_len = bfs.path().unwrap().len();
            let dijkstra_len = dijkstra.path().unwrap().len();

            assert_eq!(
                bfs_len, dijkstra_len,
                "BFS ({}) != Dijkstra ({}) for {}x{} maze",
                bfs_len, dijkstra_len, w, h
            );
        }
    }
}
