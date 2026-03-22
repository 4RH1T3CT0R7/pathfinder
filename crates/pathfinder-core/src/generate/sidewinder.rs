use super::{GenAction, GenStep, SteppableGenerator};
use crate::maze::{Direction, MazeGrid};
use crate::rng::Xoshiro256;

/// Sidewinder maze generator, implemented as a state machine.
///
/// Processes cells row by row, left to right. For each cell the algorithm randomly
/// decides to either extend the current "run" by carving East, or to close the run
/// by carving North from a randomly chosen cell within the run.
///
/// The top row (row 0) always carves East because there is no North neighbor.
/// This produces a characteristic bias: the top row has no dead-ends.
pub struct SidewinderGenerator {
    width: u32,
    height: u32,
    /// Current cell index being processed (column-major within row iteration).
    current_col: u32,
    current_row: u32,
    /// Cells in the current horizontal run (stored as cell indices).
    run: Vec<u32>,
    done: bool,
}

impl SidewinderGenerator {
    pub fn new(width: u32, height: u32) -> Self {
        SidewinderGenerator {
            width,
            height,
            current_col: 0,
            current_row: 0,
            run: Vec::new(),
            done: false,
        }
    }
}

impl SteppableGenerator for SidewinderGenerator {
    fn step(&mut self, maze: &mut dyn MazeGrid, rng: &mut Xoshiro256) -> Option<GenStep> {
        if self.done {
            return None;
        }

        let col = self.current_col;
        let row = self.current_row;
        let cell = maze.cell_index(col, row);

        // Add current cell to the run.
        self.run.push(cell);

        let is_top_row = row == 0;
        let is_last_col = col == self.width - 1;

        // Decide whether to carve East (extend the run) or close the run (carve North).
        // On the top row we must always carve East (no North neighbor), except at the last column.
        // At the last column we must close the run (no East neighbor).
        let should_close_run = if is_top_row {
            // Top row: only close at the end of the row (nowhere to carve).
            is_last_col
        } else if is_last_col {
            // Last column: must close (can't carve East).
            true
        } else {
            // Interior cell below top row: randomly decide.
            rng.next_bound(2) == 0
        };

        let step = if should_close_run {
            if is_top_row {
                // Top row, last column: nothing to carve, just visit.
                GenStep {
                    cell,
                    action: GenAction::Visit,
                }
            } else {
                // Close the run: pick a random cell from the run and carve North.
                let run_len = self.run.len() as u32;
                let pick_idx = rng.next_bound(run_len) as usize;
                let picked_cell = self.run[pick_idx];
                maze.remove_wall(picked_cell, Direction::NORTH);
                self.run.clear();
                GenStep {
                    cell: picked_cell,
                    action: GenAction::RemoveWall(Direction::NORTH),
                }
            }
        } else {
            // Extend the run: carve East.
            maze.remove_wall(cell, Direction::EAST);
            GenStep {
                cell,
                action: GenAction::RemoveWall(Direction::EAST),
            }
        };

        // Advance to the next cell.
        self.current_col += 1;
        if self.current_col >= self.width {
            self.current_col = 0;
            self.current_row += 1;
            self.run.clear();
            if self.current_row >= self.height {
                self.done = true;
            }
        }

        Some(step)
    }

    fn reset(&mut self, _start_cell: u32) {
        self.current_col = 0;
        self.current_row = 0;
        self.run.clear();
        self.done = false;
    }

    fn is_done(&self) -> bool {
        self.done
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maze::RectGrid;

    /// Helper: count open passages by checking East and South walls only (avoids double-counting).
    fn count_passages(grid: &RectGrid) -> u32 {
        let mut count = 0;
        for cell in 0..grid.cell_count() {
            let (col, row) = grid.cell_coords(cell);
            if col < grid.width() - 1 && !grid.has_wall(cell, Direction::EAST) {
                count += 1;
            }
            if row < grid.height() - 1 && !grid.has_wall(cell, Direction::SOUTH) {
                count += 1;
            }
        }
        count
    }

    /// Helper: BFS reachability check from cell 0. Returns the number of reachable cells.
    fn bfs_reachable(grid: &RectGrid) -> u32 {
        let total = grid.cell_count();
        let mut visited = vec![false; total as usize];
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(0u32);
        visited[0] = true;
        let mut count = 1u32;

        while let Some(cell) = queue.pop_front() {
            for (n, dir) in grid.neighbors(cell) {
                if !grid.has_wall(cell, dir) && !visited[n as usize] {
                    visited[n as usize] = true;
                    count += 1;
                    queue.push_back(n);
                }
            }
        }
        count
    }

    #[test]
    fn test_sidewinder_generates_perfect_maze() {
        let mut grid = RectGrid::new(10, 10);
        let mut rng = Xoshiro256::new(42);
        let mut gen = SidewinderGenerator::new(10, 10);

        let mut steps = 0;
        while let Some(_step) = gen.step(&mut grid, &mut rng) {
            steps += 1;
        }
        assert!(gen.is_done());
        assert_eq!(steps, 100); // One step per cell.

        // A perfect maze on a 10x10 grid has exactly 99 passages (N*M - 1).
        assert_eq!(count_passages(&grid), 99);
    }

    #[test]
    fn test_sidewinder_deterministic() {
        let mut grid1 = RectGrid::new(5, 5);
        let mut grid2 = RectGrid::new(5, 5);
        let mut rng1 = Xoshiro256::new(123);
        let mut rng2 = Xoshiro256::new(123);
        let mut gen1 = SidewinderGenerator::new(5, 5);
        let mut gen2 = SidewinderGenerator::new(5, 5);

        while let (Some(s1), Some(s2)) =
            (gen1.step(&mut grid1, &mut rng1), gen2.step(&mut grid2, &mut rng2))
        {
            assert_eq!(s1.cell, s2.cell);
        }

        // Verify both grids are identical.
        for cell in 0..25 {
            assert_eq!(grid1.wall_bits(cell), grid2.wall_bits(cell));
        }
    }

    #[test]
    fn test_sidewinder_all_cells_reachable() {
        let mut grid = RectGrid::new(8, 8);
        let mut rng = Xoshiro256::new(77);
        let mut gen = SidewinderGenerator::new(8, 8);

        while gen.step(&mut grid, &mut rng).is_some() {}

        assert_eq!(bfs_reachable(&grid), 64, "Not all cells are reachable");
    }

    #[test]
    fn test_sidewinder_top_row_fully_connected() {
        // The top row should be a single corridor (all East walls removed).
        let mut grid = RectGrid::new(6, 6);
        let mut rng = Xoshiro256::new(999);
        let mut gen = SidewinderGenerator::new(6, 6);

        while gen.step(&mut grid, &mut rng).is_some() {}

        for col in 0..5 {
            let cell = grid.cell_index(col, 0);
            assert!(
                !grid.has_wall(cell, Direction::EAST),
                "Top row cell ({col}, 0) should have East wall removed"
            );
        }
    }

    #[test]
    fn test_sidewinder_various_sizes() {
        for (w, h) in [(1, 1), (2, 2), (3, 7), (15, 10), (1, 10), (10, 1)] {
            let mut grid = RectGrid::new(w, h);
            let mut rng = Xoshiro256::new(w as u64 * 100 + h as u64);
            let mut gen = SidewinderGenerator::new(w, h);

            while gen.step(&mut grid, &mut rng).is_some() {}

            assert!(gen.is_done());
            let expected_passages = w * h - 1;
            assert_eq!(
                count_passages(&grid),
                expected_passages,
                "Wrong passage count for {w}x{h} grid"
            );
            assert_eq!(
                bfs_reachable(&grid),
                w * h,
                "Not all cells reachable in {w}x{h} grid"
            );
        }
    }

    #[test]
    fn test_sidewinder_reset() {
        let mut grid = RectGrid::new(5, 5);
        let mut rng = Xoshiro256::new(42);
        let mut gen = SidewinderGenerator::new(5, 5);

        // Run to completion.
        while gen.step(&mut grid, &mut rng).is_some() {}
        assert!(gen.is_done());

        // Reset and verify it can run again on a fresh grid.
        gen.reset(0);
        assert!(!gen.is_done());

        let mut grid2 = RectGrid::new(5, 5);
        let mut rng2 = Xoshiro256::new(42);
        while gen.step(&mut grid2, &mut rng2).is_some() {}
        assert!(gen.is_done());
        assert_eq!(count_passages(&grid2), 24);
    }
}
