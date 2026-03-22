use super::{SolveAction, SolveStep, SteppableSolver};
use crate::maze::MazeGrid;

/// DFS maze solver implemented as a state machine using an explicit stack.
///
/// Depth-first search explores as far as possible along each branch before
/// backtracking. It does **not** guarantee the shortest path, but uses less
/// memory than BFS in practice for deep mazes.
pub struct DfsSolver {
    stack: Vec<u32>,
    visited: Vec<bool>,
    parent: Vec<Option<u32>>,
    start: u32,
    end: u32,
    done: bool,
    found: bool,
    solution_path: Vec<u32>,
}

impl DfsSolver {
    pub fn new(cell_count: u32, start: u32, end: u32) -> Self {
        let mut visited = vec![false; cell_count as usize];
        visited[start as usize] = true;
        let stack = vec![start];

        DfsSolver {
            stack,
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

impl SteppableSolver for DfsSolver {
    fn step(&mut self, maze: &dyn MazeGrid) -> Option<SolveStep> {
        if self.done {
            return None;
        }

        let current = match self.stack.pop() {
            Some(cell) => cell,
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

        // Explore neighbors through open passages
        for (neighbor, dir) in maze.neighbors(current) {
            if !maze.has_wall(current, dir) && !self.visited[neighbor as usize] {
                self.visited[neighbor as usize] = true;
                self.parent[neighbor as usize] = Some(current);
                self.stack.push(neighbor);
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
        self.visited.fill(false);
        self.visited[start as usize] = true;
        self.parent.fill(None);
        self.stack.clear();
        self.stack.push(start);
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
    fn test_dfs_finds_path() {
        let grid = generate_maze(10, 10, 42);
        let start = 0;
        let end = grid.cell_count() - 1;
        let mut solver = DfsSolver::new(grid.cell_count(), start, end);

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
    fn test_dfs_path_is_valid() {
        let grid = generate_maze(8, 8, 99);
        let start = 0;
        let end = 63;
        let mut solver = DfsSolver::new(grid.cell_count(), start, end);

        while solver.step(&grid).is_some() {}

        let path = solver.path().unwrap();
        // Verify each step in path is adjacent and has no wall between
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
    fn test_dfs_start_equals_end() {
        let grid = generate_maze(5, 5, 1);
        let mut solver = DfsSolver::new(grid.cell_count(), 0, 0);

        let step = solver.step(&grid);
        assert!(step.is_some());
        assert!(matches!(step.unwrap().action, SolveAction::FoundGoal));
        assert!(solver.is_done());
        assert_eq!(solver.path().unwrap(), &[0]);
    }

    #[test]
    fn test_dfs_finds_path_various_seeds() {
        for seed in 0..20 {
            let grid = generate_maze(12, 12, seed);
            let start = 0;
            let end = grid.cell_count() - 1;
            let mut solver = DfsSolver::new(grid.cell_count(), start, end);

            while solver.step(&grid).is_some() {}

            assert!(
                solver.path().is_some(),
                "DFS failed to find a path for seed {}",
                seed
            );
            let path = solver.path().unwrap();
            assert_eq!(path[0], start);
            assert_eq!(*path.last().unwrap(), end);
        }
    }

    #[test]
    fn test_dfs_reset_works() {
        let grid = generate_maze(6, 6, 42);
        let cell_count = grid.cell_count();
        let mut solver = DfsSolver::new(cell_count, 0, cell_count - 1);

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
    fn test_dfs_on_corridor() {
        // A simple 3x1 corridor: DFS should still find the path [0, 1, 2]
        let mut grid = RectGrid::new(3, 1);
        grid.remove_wall(0, Direction::EAST);
        grid.remove_wall(1, Direction::EAST);

        let mut solver = DfsSolver::new(3, 0, 2);
        while solver.step(&grid).is_some() {}

        let path = solver.path().unwrap();
        assert_eq!(path, &[0, 1, 2]);
    }

    #[test]
    fn test_dfs_bfs_both_find_path() {
        // DFS may not find shortest path, but it must find *a* path in the same maze
        let grid = generate_maze(10, 10, 77);
        let start = 0;
        let end = grid.cell_count() - 1;

        let mut bfs = BfsSolver::new(grid.cell_count(), start, end);
        while bfs.step(&grid).is_some() {}

        let mut dfs = DfsSolver::new(grid.cell_count(), start, end);
        while dfs.step(&grid).is_some() {}

        assert!(bfs.path().is_some());
        assert!(dfs.path().is_some());

        // DFS path may be longer or equal, but never shorter than BFS
        let bfs_len = bfs.path().unwrap().len();
        let dfs_len = dfs.path().unwrap().len();
        assert!(
            dfs_len >= bfs_len,
            "DFS path ({}) should not be shorter than BFS path ({})",
            dfs_len,
            bfs_len
        );
    }
}
