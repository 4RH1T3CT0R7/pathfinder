use std::collections::HashMap;
use super::{SolveAction, SolveStep, SteppableSolver};
use crate::maze::{Direction, MazeGrid};

/// Tremaux's algorithm maze solver.
///
/// Each passage (cell, direction) is marked 0, 1, or 2 times. The rules are:
///
/// 1. At a junction, prefer an unvisited passage (mark = 0).
/// 2. If all passages have been visited at least once, go back the way you
///    came (marking that passage a second time).
/// 3. Never enter a passage marked twice.
///
/// This algorithm works for **all** mazes, including those with loops, and
/// always finds *a* path (not necessarily the shortest).
pub struct TremauxSolver {
    current: u32,
    start: u32,
    end: u32,
    /// Number of marks on passage (cell, direction). 0, 1, or 2.
    marks: HashMap<(u32, Direction), u8>,
    /// The direction we arrived from (from the *previous* cell's perspective).
    /// `None` for the very first step.
    came_from_dir: Option<Direction>,
    done: bool,
    found: bool,
    /// Trail of cells in visit order.
    trail: Vec<u32>,
    /// Cleaned solution path.
    solution_path: Vec<u32>,
    started: bool,
}

impl TremauxSolver {
    pub fn new(_cell_count: u32, start: u32, end: u32) -> Self {
        TremauxSolver {
            current: start,
            start,
            end,
            marks: HashMap::new(),
            came_from_dir: None,
            done: false,
            found: false,
            trail: Vec::new(),
            solution_path: Vec::new(),
            started: false,
        }
    }

    fn get_mark(&self, cell: u32, dir: Direction) -> u8 {
        self.marks.get(&(cell, dir)).copied().unwrap_or(0)
    }

    fn add_mark(&mut self, cell: u32, dir: Direction) {
        let entry = self.marks.entry((cell, dir)).or_insert(0);
        *entry = (*entry + 1).min(2);
    }

    /// Get all passable directions from `cell` (no wall, neighbor exists).
    fn passable_dirs(cell: u32, maze: &dyn MazeGrid) -> Vec<(Direction, u32)> {
        let mut dirs = Vec::new();
        for (neighbor, dir) in maze.neighbors(cell) {
            if !maze.has_wall(cell, dir) {
                dirs.push((dir, neighbor));
            }
        }
        dirs
    }

    /// Extract a clean path from the trail by removing loops.
    fn extract_path(&mut self) {
        self.solution_path.clear();
        let len = self.trail.len();
        if len == 0 {
            return;
        }

        let max_cell = self.trail.iter().copied().max().unwrap_or(0) as usize;
        let mut last_index = vec![0usize; max_cell + 1];
        for (i, &cell) in self.trail.iter().enumerate() {
            last_index[cell as usize] = i;
        }

        let mut i = 0;
        while i < len {
            let cell = self.trail[i];
            self.solution_path.push(cell);
            if cell == self.end {
                break;
            }
            let li = last_index[cell as usize];
            i = li + 1;
        }
    }
}

impl SteppableSolver for TremauxSolver {
    fn step(&mut self, maze: &dyn MazeGrid) -> Option<SolveStep> {
        if self.done {
            return None;
        }

        // First call: emit start cell.
        if !self.started {
            self.started = true;
            self.trail.push(self.current);

            if self.current == self.end {
                self.done = true;
                self.found = true;
                self.solution_path.push(self.current);
                return Some(SolveStep {
                    cell: self.current,
                    action: SolveAction::FoundGoal,
                });
            }

            return Some(SolveStep {
                cell: self.current,
                action: SolveAction::Visit,
            });
        }

        let passable = Self::passable_dirs(self.current, maze);
        if passable.is_empty() {
            // Completely isolated cell.
            self.done = true;
            return None;
        }

        // Separate directions into categories.
        let mut unvisited: Vec<Direction> = Vec::new();
        let mut once: Vec<Direction> = Vec::new();

        for &(dir, _) in &passable {
            let mark = self.get_mark(self.current, dir);
            match mark {
                0 => unvisited.push(dir),
                1 => once.push(dir),
                _ => {} // mark == 2, skip
            }
        }

        // Tremaux rules:
        // 1. If there is an unvisited passage, take it.
        // 2. Otherwise, if we came from somewhere, go back the way we came
        //    (the opposite direction of how we arrived) -- this marks it twice.
        // 3. Otherwise, take any passage with mark 1.
        // 4. If nothing is available, we are stuck.
        let chosen_dir = if !unvisited.is_empty() {
            // Prefer an unvisited passage. Pick the first one that is NOT the
            // way we came (to avoid immediate backtrack when there are other
            // options). If all unvisited are the way we came, just take it.
            let back_dir = self.came_from_dir.map(|d| maze.opposite(d));
            unvisited
                .iter()
                .find(|&&d| Some(d) != back_dir)
                .or(unvisited.first())
                .copied()
        } else if let Some(back) = self.came_from_dir.map(|d| maze.opposite(d)) {
            // Go back the way we came if that passage has mark < 2.
            if self.get_mark(self.current, back) < 2
                && passable.iter().any(|&(d, _)| d == back)
            {
                Some(back)
            } else {
                // Take any passage with mark 1.
                once.first().copied()
            }
        } else {
            once.first().copied()
        };

        let dir = match chosen_dir {
            Some(d) => d,
            None => {
                // No passable direction available; maze is unsolvable from here.
                self.done = true;
                return None;
            }
        };

        // Mark the passage from this side.
        self.add_mark(self.current, dir);

        // Move to the neighbor.
        let next = match passable.iter().find(|&&(d, _)| d == dir).map(|&(_, n)| n) {
            Some(n) => n,
            None => {
                self.done = true;
                return None;
            }
        };

        // Mark the passage from the neighbor's side (opposite direction).
        self.add_mark(next, maze.opposite(dir));

        self.came_from_dir = Some(dir);
        self.current = next;
        self.trail.push(self.current);

        if self.current == self.end {
            self.done = true;
            self.found = true;
            self.extract_path();
            return Some(SolveStep {
                cell: self.current,
                action: SolveAction::FoundGoal,
            });
        }

        Some(SolveStep {
            cell: self.current,
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
        self.current = start;
        self.start = start;
        self.end = end;
        self.marks.clear();
        self.came_from_dir = None;
        self.done = false;
        self.found = false;
        self.trail.clear();
        self.solution_path.clear();
        self.started = false;
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
    fn test_tremaux_finds_path() {
        let grid = generate_maze(10, 10, 42);
        let start = 0;
        let end = grid.cell_count() - 1;
        let mut solver = TremauxSolver::new(grid.cell_count(), start, end);

        let mut steps = 0;
        while let Some(_step) = solver.step(&grid) {
            steps += 1;
            assert!(steps < 10_000, "Tremaux did not terminate");
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
    fn test_tremaux_path_is_valid() {
        let grid = generate_maze(8, 8, 99);
        let start = 0;
        let end = 63;
        let mut solver = TremauxSolver::new(grid.cell_count(), start, end);

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
    fn test_tremaux_start_equals_end() {
        let grid = generate_maze(5, 5, 1);
        let mut solver = TremauxSolver::new(grid.cell_count(), 0, 0);

        let step = solver.step(&grid);
        assert!(step.is_some());
        assert!(matches!(step.unwrap().action, SolveAction::FoundGoal));
        assert!(solver.is_done());
        assert_eq!(solver.path().unwrap(), &[0]);
    }

    #[test]
    fn test_tremaux_various_sizes() {
        for &(w, h, seed) in &[(5, 5, 10), (15, 15, 77), (20, 10, 200)] {
            let grid = generate_maze(w, h, seed);
            let start = 0;
            let end = grid.cell_count() - 1;
            let mut solver = TremauxSolver::new(grid.cell_count(), start, end);

            let mut steps = 0;
            while solver.step(&grid).is_some() {
                steps += 1;
                assert!(steps < 50_000, "Tremaux did not terminate for {}x{}", w, h);
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
    fn test_tremaux_simple_corridor() {
        let mut grid = RectGrid::new(4, 1);
        grid.remove_wall(0, Direction::EAST);
        grid.remove_wall(1, Direction::EAST);
        grid.remove_wall(2, Direction::EAST);

        let mut solver = TremauxSolver::new(4, 0, 3);
        while solver.step(&grid).is_some() {}

        let path = solver.path().unwrap();
        assert_eq!(path, &[0, 1, 2, 3]);
    }

    #[test]
    fn test_tremaux_reset() {
        let grid = generate_maze(6, 6, 55);
        let mut solver = TremauxSolver::new(grid.cell_count(), 0, 35);

        let mut steps = 0;
        while solver.step(&grid).is_some() {
            steps += 1;
            assert!(steps < 50_000);
        }
        assert!(solver.path().is_some());

        solver.reset(5, 30);
        assert!(!solver.is_done());
        assert!(solver.path().is_none());

        steps = 0;
        while solver.step(&grid).is_some() {
            steps += 1;
            assert!(steps < 50_000);
        }
        assert!(solver.is_done());
        assert!(solver.path().is_some());
        let path = solver.path().unwrap();
        assert_eq!(path[0], 5);
        assert_eq!(*path.last().unwrap(), 30);
    }

    #[test]
    fn test_tremaux_marks_capped_at_two() {
        let mut solver = TremauxSolver::new(4, 0, 3);
        solver.add_mark(0, Direction::EAST);
        assert_eq!(solver.get_mark(0, Direction::EAST), 1);
        solver.add_mark(0, Direction::EAST);
        assert_eq!(solver.get_mark(0, Direction::EAST), 2);
        solver.add_mark(0, Direction::EAST);
        // Should not exceed 2.
        assert_eq!(solver.get_mark(0, Direction::EAST), 2);
    }
}
