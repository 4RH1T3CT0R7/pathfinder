use std::collections::VecDeque;
use super::{SolveAction, SolveStep, SteppableSolver};
use crate::maze::MazeGrid;

/// Dead-End Filling maze solver.
///
/// Operates in two phases:
///
/// **Phase 1 -- Fill dead ends:** Iteratively find cells with exactly one open
/// passage (dead ends) and mark them as filled. When a dead end is filled its
/// neighbor may become a new dead end, so the process continues until no dead
/// ends remain. The start and end cells are never filled.
///
/// **Phase 2 -- Trace path:** Once all dead ends are filled the remaining
/// unfilled cells form the solution corridor. A simple BFS from start to end
/// through unfilled cells recovers the path.
///
/// This algorithm works well for perfect (simply-connected) mazes.
pub struct DeadEndFillingSolver {
    start: u32,
    end: u32,
    cell_count: u32,
    done: bool,
    found: bool,

    /// `true` for cells that have been filled (dead-end eliminated).
    filled: Vec<bool>,

    /// Queue of cells to check for dead-end status.
    fill_queue: VecDeque<u32>,

    /// Current phase of the algorithm.
    phase: Phase,

    /// BFS state for phase 2 path tracing.
    trace_queue: VecDeque<u32>,
    trace_visited: Vec<bool>,
    trace_parent: Vec<Option<u32>>,

    /// The final solution path.
    solution_path: Vec<u32>,

    /// Whether phase 1 has been initialized.
    initialized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    FillDeadEnds,
    TracePath,
}

impl DeadEndFillingSolver {
    pub fn new(cell_count: u32, start: u32, end: u32) -> Self {
        DeadEndFillingSolver {
            start,
            end,
            cell_count,
            done: false,
            found: false,
            filled: vec![false; cell_count as usize],
            fill_queue: VecDeque::new(),
            phase: Phase::FillDeadEnds,
            trace_queue: VecDeque::new(),
            trace_visited: vec![false; cell_count as usize],
            trace_parent: vec![None; cell_count as usize],
            solution_path: Vec::new(),
            initialized: false,
        }
    }

    /// Count the number of open (no wall, not filled) passages from a cell.
    fn open_passage_count(cell: u32, maze: &dyn MazeGrid, filled: &[bool]) -> u32 {
        let mut count = 0;
        for (neighbor, dir) in maze.neighbors(cell) {
            if !maze.has_wall(cell, dir) && !filled[neighbor as usize] {
                count += 1;
            }
        }
        count
    }

    /// Check if a cell is a dead end (exactly 1 open passage) and is not
    /// the start or end cell.
    fn is_dead_end(&self, cell: u32, maze: &dyn MazeGrid) -> bool {
        if cell == self.start || cell == self.end {
            return false;
        }
        if self.filled[cell as usize] {
            return false;
        }
        Self::open_passage_count(cell, maze, &self.filled) <= 1
    }

    fn reconstruct_trace_path(&mut self) {
        self.solution_path.clear();
        let mut current = self.end;
        self.solution_path.push(current);
        while current != self.start {
            match self.trace_parent[current as usize] {
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

impl SteppableSolver for DeadEndFillingSolver {
    fn step(&mut self, maze: &dyn MazeGrid) -> Option<SolveStep> {
        if self.done {
            return None;
        }

        // Handle start == end immediately.
        if !self.initialized && self.start == self.end {
            self.initialized = true;
            self.done = true;
            self.found = true;
            self.solution_path.push(self.start);
            return Some(SolveStep {
                cell: self.start,
                action: SolveAction::FoundGoal,
            });
        }

        // Initialize phase 1: seed the queue with all current dead ends.
        if !self.initialized {
            self.initialized = true;
            for cell in 0..self.cell_count {
                if self.is_dead_end(cell, maze) {
                    self.fill_queue.push_back(cell);
                }
            }
        }

        match self.phase {
            Phase::FillDeadEnds => {
                // Process one dead-end cell per step.
                while let Some(cell) = self.fill_queue.pop_front() {
                    // Re-check: the cell might no longer be a dead end.
                    if self.filled[cell as usize] || !self.is_dead_end(cell, maze) {
                        continue;
                    }

                    // Fill this dead end.
                    self.filled[cell as usize] = true;

                    // Check if filling this cell caused any of its structural
                    // neighbors to become new dead ends.
                    let structural_neighbors = maze.neighbors(cell);
                    for &(neighbor, dir) in &structural_neighbors {
                        if !maze.has_wall(cell, dir)
                            && !self.filled[neighbor as usize]
                            && self.is_dead_end(neighbor, maze)
                        {
                            self.fill_queue.push_back(neighbor);
                        }
                    }

                    return Some(SolveStep {
                        cell,
                        action: SolveAction::Visit,
                    });
                }

                // No more dead ends. Transition to phase 2.
                self.phase = Phase::TracePath;
                self.trace_visited[self.start as usize] = true;
                self.trace_queue.push_back(self.start);

                // Fall through to phase 2 on next call. Emit a marker step
                // for the start cell.
                Some(SolveStep {
                    cell: self.start,
                    action: SolveAction::AddToFrontier,
                })
            }
            Phase::TracePath => {
                let current = match self.trace_queue.pop_front() {
                    Some(c) => c,
                    None => {
                        // Could not reach end through unfilled cells.
                        self.done = true;
                        return None;
                    }
                };

                if current == self.end {
                    self.done = true;
                    self.found = true;
                    self.reconstruct_trace_path();
                    return Some(SolveStep {
                        cell: current,
                        action: SolveAction::FoundGoal,
                    });
                }

                // BFS through unfilled cells.
                for (neighbor, dir) in maze.neighbors(current) {
                    if !maze.has_wall(current, dir)
                        && !self.filled[neighbor as usize]
                        && !self.trace_visited[neighbor as usize]
                    {
                        self.trace_visited[neighbor as usize] = true;
                        self.trace_parent[neighbor as usize] = Some(current);
                        self.trace_queue.push_back(neighbor);
                    }
                }

                Some(SolveStep {
                    cell: current,
                    action: SolveAction::MarkPath,
                })
            }
        }
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
        self.done = false;
        self.found = false;
        self.filled.fill(false);
        self.fill_queue.clear();
        self.phase = Phase::FillDeadEnds;
        self.trace_queue.clear();
        self.trace_visited.fill(false);
        self.trace_parent.fill(None);
        self.solution_path.clear();
        self.initialized = false;
    }

    fn is_done(&self) -> bool {
        self.done
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maze::{Direction, RectGrid};
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
    fn test_dead_end_filling_finds_path() {
        let grid = generate_maze(10, 10, 42);
        let start = 0;
        let end = grid.cell_count() - 1;
        let mut solver = DeadEndFillingSolver::new(grid.cell_count(), start, end);

        let mut steps = 0;
        while let Some(_step) = solver.step(&grid) {
            steps += 1;
            assert!(steps < 10_000, "Dead-end filling did not terminate");
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
    fn test_dead_end_filling_path_is_valid() {
        let grid = generate_maze(8, 8, 99);
        let start = 0;
        let end = 63;
        let mut solver = DeadEndFillingSolver::new(grid.cell_count(), start, end);

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
    fn test_dead_end_filling_start_equals_end() {
        let grid = generate_maze(5, 5, 1);
        let mut solver = DeadEndFillingSolver::new(grid.cell_count(), 0, 0);

        let step = solver.step(&grid);
        assert!(step.is_some());
        assert!(matches!(step.unwrap().action, SolveAction::FoundGoal));
        assert!(solver.is_done());
        assert_eq!(solver.path().unwrap(), &[0]);
    }

    #[test]
    fn test_dead_end_filling_various_sizes() {
        for &(w, h, seed) in &[(5, 5, 10), (15, 15, 77), (20, 10, 200)] {
            let grid = generate_maze(w, h, seed);
            let start = 0;
            let end = grid.cell_count() - 1;
            let mut solver = DeadEndFillingSolver::new(grid.cell_count(), start, end);

            let mut steps = 0;
            while solver.step(&grid).is_some() {
                steps += 1;
                assert!(
                    steps < 50_000,
                    "Dead-end filling did not terminate for {}x{}",
                    w,
                    h
                );
            }

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
    fn test_dead_end_filling_simple_corridor() {
        let mut grid = RectGrid::new(5, 1);
        grid.remove_wall(0, Direction::EAST);
        grid.remove_wall(1, Direction::EAST);
        grid.remove_wall(2, Direction::EAST);
        grid.remove_wall(3, Direction::EAST);

        let mut solver = DeadEndFillingSolver::new(5, 0, 4);
        while solver.step(&grid).is_some() {}

        let path = solver.path().unwrap();
        assert_eq!(path, &[0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_dead_end_filling_fills_branches() {
        // Create a T-shaped maze:
        //   0 - 1 - 2
        //       |
        //       3
        // Start=0, End=2. Cell 3 is a dead end that should be filled.
        let mut grid = RectGrid::new(3, 2);
        // Grid layout (3 wide, 2 tall):
        //  0  1  2
        //  3  4  5
        // Open: 0-1, 1-2, 1-4 (via South from 1)
        grid.remove_wall(0, Direction::EAST); // 0 <-> 1
        grid.remove_wall(1, Direction::EAST); // 1 <-> 2
        grid.remove_wall(1, Direction::SOUTH); // 1 <-> 4

        let mut solver = DeadEndFillingSolver::new(grid.cell_count(), 0, 2);
        while solver.step(&grid).is_some() {}

        let path = solver.path().unwrap();
        // The dead-end branch (cell 4) should have been filled,
        // so the path goes 0 -> 1 -> 2.
        assert_eq!(path, &[0, 1, 2]);
    }

    #[test]
    fn test_dead_end_filling_reset() {
        let grid = generate_maze(6, 6, 55);
        let mut solver = DeadEndFillingSolver::new(grid.cell_count(), 0, 35);

        while solver.step(&grid).is_some() {}
        assert!(solver.path().is_some());

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

    #[test]
    fn test_dead_end_filling_produces_shortest_in_perfect_maze() {
        // In a perfect (tree) maze, dead-end filling should find the unique
        // path, which is also the shortest. Compare against BFS.
        use crate::solve::BfsSolver;

        let grid = generate_maze(10, 10, 42);
        let start = 0;
        let end = grid.cell_count() - 1;

        let mut bfs = BfsSolver::new(grid.cell_count(), start, end);
        while bfs.step(&grid).is_some() {}
        let bfs_path = bfs.path().unwrap();

        let mut def = DeadEndFillingSolver::new(grid.cell_count(), start, end);
        while def.step(&grid).is_some() {}
        let def_path = def.path().unwrap();

        assert_eq!(
            bfs_path, def_path,
            "Dead-end filling should find the same unique path as BFS in a perfect maze"
        );
    }
}
