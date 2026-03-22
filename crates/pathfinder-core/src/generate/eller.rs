use super::{GenAction, GenStep, SteppableGenerator};
use crate::maze::{Direction, MazeGrid};
use crate::rng::Xoshiro256;

/// Eller's algorithm maze generator, implemented as a state machine.
///
/// Eller's algorithm processes the maze one row at a time, maintaining a
/// disjoint-set structure over the current row's cells. For each row it
/// performs two phases:
///
/// 1. **Horizontal merging** -- iterate left to right; for each pair of
///    adjacent cells that belong to different sets, randomly decide whether
///    to remove the wall between them (merging their sets). On the *last* row,
///    all adjacent cells in different sets are always merged (to ensure a
///    perfect maze).
///
/// 2. **Vertical connections** -- for each set in the current row, randomly
///    select at least one cell to connect downward to the next row. Cells that
///    are connected keep their set id in the next row; cells that are not
///    connected start a new set.
///
/// Each call to `step()` performs one atomic operation (one horizontal merge
/// decision or one vertical connection decision) so that the caller can
/// animate the generation process.
pub struct EllerGenerator {
    width: u32,
    height: u32,
    /// Per-cell set id for the *current* row.
    set_id: Vec<u32>,
    /// Monotonically increasing counter to assign fresh set ids.
    next_set_id: u32,
    /// Current row being processed (0-indexed).
    current_row: u32,
    /// Current phase within the row.
    phase: Phase,
    /// Column cursor within the current phase.
    col_cursor: u32,
    /// Whether generation is complete.
    done: bool,
    /// Whether the generator has been initialised (deferred to first `step`).
    initialised: bool,
    /// For the vertical phase: tracks which sets have already sent at least
    /// one cell downward. Indexed by column; set to `true` once a cell in
    /// that column's set has been connected down. We use a separate vec
    /// (`set_has_down`) keyed by *set id* for correct tracking.
    set_has_down: Vec<bool>,
    /// Maximum set id we might see (used to size `set_has_down`).
    set_has_down_cap: usize,
    /// For the vertical-finalise sub-phase: column cursor.
    vert_final_cursor: u32,
}

/// Internal phase within a single row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Horizontal merge: deciding whether to join cell `col_cursor` with
    /// cell `col_cursor + 1`.
    Horizontal,
    /// Vertical connections: deciding whether to carve downward from
    /// `col_cursor`.
    Vertical,
    /// After individual vertical decisions, ensure every set has at least
    /// one downward connection (guarantee connectivity).
    VerticalFinalize,
    /// Prepare the next row's set ids based on vertical connections made.
    AdvanceRow,
}

impl EllerGenerator {
    pub fn new(_cell_count: u32, _start_cell: u32) -> Self {
        // Width and height are not known until first `step` call (we read
        // them from the MazeGrid). We allocate a set_id vec large enough
        // for the full grid width but only fill it during init.
        EllerGenerator {
            width: 0,
            height: 0,
            set_id: Vec::new(),
            next_set_id: 0,
            current_row: 0,
            phase: Phase::Horizontal,
            col_cursor: 0,
            done: false,
            initialised: false,
            set_has_down: Vec::new(),
            set_has_down_cap: 0,
            vert_final_cursor: 0,
        }
    }

    fn init(&mut self, maze: &dyn MazeGrid) {
        self.width = maze.width();
        self.height = maze.height();
        let w = self.width as usize;
        // Each cell in the first row gets its own set.
        self.set_id = (0..self.width).collect();
        self.next_set_id = self.width;
        self.current_row = 0;
        self.phase = Phase::Horizontal;
        self.col_cursor = 0;
        // Upper bound on set ids we will ever use. Each row can create at
        // most `width` new sets, and there are `height` rows.
        self.set_has_down_cap = (self.width as usize) * (self.height as usize) + w;
        self.set_has_down = vec![false; self.set_has_down_cap];
        self.initialised = true;
    }

    /// Merge all cells in `set_id` that have old_id into new_id (within
    /// the current row only).
    fn merge_sets(&mut self, old_id: u32, new_id: u32) {
        for s in &mut self.set_id {
            if *s == old_id {
                *s = new_id;
            }
        }
    }

    /// Ensure `set_has_down` is large enough for `id`.
    fn ensure_set_has_down(&mut self, id: u32) {
        let idx = id as usize;
        if idx >= self.set_has_down.len() {
            self.set_has_down.resize(idx + 1, false);
        }
    }
}

impl SteppableGenerator for EllerGenerator {
    fn step(&mut self, maze: &mut dyn MazeGrid, rng: &mut Xoshiro256) -> Option<GenStep> {
        if self.done {
            return None;
        }

        if !self.initialised {
            self.init(maze);
        }

        if self.height == 0 || self.width == 0 {
            self.done = true;
            return None;
        }

        // Loop instead of recursion to avoid stack overflow in WASM.
        loop {
            match self.phase {
                Phase::Horizontal => {
                    if self.width <= 1 || self.col_cursor >= self.width - 1 {
                        if self.current_row == self.height - 1 {
                            self.done = true;
                            return None;
                        }
                        self.phase = Phase::Vertical;
                        self.col_cursor = 0;
                        for col in 0..self.width {
                            let sid = self.set_id[col as usize];
                            self.ensure_set_has_down(sid);
                            self.set_has_down[sid as usize] = false;
                        }
                        continue;
                    }

                    let col = self.col_cursor;
                    self.col_cursor += 1;

                    let left_set = self.set_id[col as usize];
                    let right_set = self.set_id[(col + 1) as usize];

                    if left_set == right_set {
                        continue;
                    }

                    let is_last_row = self.current_row == self.height - 1;
                    let should_merge = is_last_row || rng.next_bound(2) == 0;

                    if should_merge {
                        let cell = maze.cell_index(col, self.current_row);
                        maze.remove_wall(cell, Direction::EAST);
                        self.merge_sets(right_set, left_set);
                        return Some(GenStep {
                            cell,
                            action: GenAction::RemoveWall(Direction::EAST),
                        });
                    }

                    continue;
                }

                Phase::Vertical => {
                    if self.col_cursor >= self.width {
                        self.phase = Phase::VerticalFinalize;
                        self.vert_final_cursor = 0;
                        continue;
                    }

                    let col = self.col_cursor;
                    self.col_cursor += 1;

                    let sid = self.set_id[col as usize];
                    self.ensure_set_has_down(sid);

                    let carve = rng.next_bound(2) == 0;

                    if carve {
                        self.set_has_down[sid as usize] = true;
                        let cell = maze.cell_index(col, self.current_row);
                        maze.remove_wall(cell, Direction::SOUTH);
                        return Some(GenStep {
                            cell,
                            action: GenAction::RemoveWall(Direction::SOUTH),
                        });
                    }

                    continue;
                }

                Phase::VerticalFinalize => {
                    while self.vert_final_cursor < self.width {
                        let col = self.vert_final_cursor;
                        self.vert_final_cursor += 1;

                        let sid = self.set_id[col as usize];
                        self.ensure_set_has_down(sid);

                        if !self.set_has_down[sid as usize] {
                            self.set_has_down[sid as usize] = true;
                            let cell = maze.cell_index(col, self.current_row);
                            maze.remove_wall(cell, Direction::SOUTH);
                            return Some(GenStep {
                                cell,
                                action: GenAction::RemoveWall(Direction::SOUTH),
                            });
                        }
                    }

                    self.phase = Phase::AdvanceRow;
                    continue;
                }

                Phase::AdvanceRow => {
                    let row = self.current_row;
                    for col in 0..self.width {
                        let cell = maze.cell_index(col, row);
                        if maze.has_wall(cell, Direction::SOUTH) {
                            self.set_id[col as usize] = self.next_set_id;
                            self.next_set_id += 1;
                        }
                    }

                    self.current_row += 1;
                    self.col_cursor = 0;
                    self.phase = Phase::Horizontal;
                    continue;
                }
            }
        }
    }

    fn reset(&mut self, _start_cell: u32) {
        self.set_id.clear();
        self.next_set_id = 0;
        self.current_row = 0;
        self.phase = Phase::Horizontal;
        self.col_cursor = 0;
        self.done = false;
        self.initialised = false;
        self.set_has_down.clear();
        self.vert_final_cursor = 0;
    }

    fn is_done(&self) -> bool {
        self.done
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maze::RectGrid;

    /// Count the number of open passages (removed walls) in the grid.
    fn count_passages(grid: &RectGrid) -> u32 {
        let mut count = 0;
        for cell in 0..grid.cell_count() {
            let (col, row) = grid.cell_coords(cell);
            if col < grid.width() - 1 && !grid.has_wall(cell, crate::maze::Direction::EAST) {
                count += 1;
            }
            if row < grid.height() - 1 && !grid.has_wall(cell, crate::maze::Direction::SOUTH) {
                count += 1;
            }
        }
        count
    }

    /// BFS reachability check from cell 0.
    fn all_cells_reachable(grid: &RectGrid) -> bool {
        let total = grid.cell_count() as usize;
        let mut visited = vec![false; total];
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(0u32);
        visited[0] = true;

        while let Some(cell) = queue.pop_front() {
            for (n, dir) in grid.neighbors(cell) {
                if !grid.has_wall(cell, dir) && !visited[n as usize] {
                    visited[n as usize] = true;
                    queue.push_back(n);
                }
            }
        }

        visited.iter().all(|&v| v)
    }

    #[test]
    fn test_eller_generates_perfect_maze() {
        let mut grid = RectGrid::new(10, 10);
        let mut rng = Xoshiro256::new(42);
        let mut gen = EllerGenerator::new(grid.cell_count(), 0);

        let mut steps = 0;
        while let Some(_step) = gen.step(&mut grid, &mut rng) {
            steps += 1;
        }
        assert!(gen.is_done());
        assert!(steps > 0);

        // A perfect maze on a 10x10 grid has exactly 99 removed walls.
        assert_eq!(count_passages(&grid), 99);
    }

    #[test]
    fn test_eller_deterministic() {
        let mut grid1 = RectGrid::new(5, 5);
        let mut grid2 = RectGrid::new(5, 5);
        let mut rng1 = Xoshiro256::new(123);
        let mut rng2 = Xoshiro256::new(123);
        let mut gen1 = EllerGenerator::new(25, 0);
        let mut gen2 = EllerGenerator::new(25, 0);

        while let (Some(s1), Some(s2)) =
            (gen1.step(&mut grid1, &mut rng1), gen2.step(&mut grid2, &mut rng2))
        {
            assert_eq!(s1.cell, s2.cell);
        }

        for cell in 0..25 {
            assert_eq!(grid1.wall_bits(cell), grid2.wall_bits(cell));
        }
    }

    #[test]
    fn test_eller_all_cells_reachable() {
        let mut grid = RectGrid::new(8, 8);
        let mut rng = Xoshiro256::new(77);
        let mut gen = EllerGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}

        assert!(all_cells_reachable(&grid), "Not all cells are reachable");
    }

    #[test]
    fn test_eller_small_grid() {
        let mut grid = RectGrid::new(2, 2);
        let mut rng = Xoshiro256::new(1);
        let mut gen = EllerGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}

        assert!(gen.is_done());
        assert_eq!(count_passages(&grid), 3); // 2*2 - 1
        assert!(all_cells_reachable(&grid));
    }

    #[test]
    fn test_eller_rectangular_grid() {
        let mut grid = RectGrid::new(7, 3);
        let mut rng = Xoshiro256::new(999);
        let mut gen = EllerGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}

        assert!(gen.is_done());
        assert_eq!(count_passages(&grid), 20); // 7*3 - 1
        assert!(all_cells_reachable(&grid));
    }

    #[test]
    fn test_eller_single_column() {
        // A 1xN grid should produce a straight corridor.
        let mut grid = RectGrid::new(1, 5);
        let mut rng = Xoshiro256::new(42);
        let mut gen = EllerGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}

        assert!(gen.is_done());
        assert_eq!(count_passages(&grid), 4); // 1*5 - 1
        assert!(all_cells_reachable(&grid));
    }

    #[test]
    fn test_eller_single_row() {
        // A Nx1 grid should produce a straight corridor.
        let mut grid = RectGrid::new(5, 1);
        let mut rng = Xoshiro256::new(42);
        let mut gen = EllerGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}

        assert!(gen.is_done());
        assert_eq!(count_passages(&grid), 4); // 5*1 - 1
        assert!(all_cells_reachable(&grid));
    }

    #[test]
    fn test_eller_reset() {
        let mut grid = RectGrid::new(5, 5);
        let mut rng = Xoshiro256::new(42);
        let mut gen = EllerGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}
        assert!(gen.is_done());

        // Reset and generate again on a fresh grid.
        gen.reset(0);
        assert!(!gen.is_done());

        let mut grid2 = RectGrid::new(5, 5);
        while gen.step(&mut grid2, &mut rng).is_some() {}
        assert!(gen.is_done());
        assert_eq!(count_passages(&grid2), 24);
        assert!(all_cells_reachable(&grid2));
    }

    #[test]
    fn test_eller_many_seeds() {
        // Run with many different seeds to increase confidence in correctness.
        for seed in 0..50 {
            let mut grid = RectGrid::new(6, 6);
            let mut rng = Xoshiro256::new(seed);
            let mut gen = EllerGenerator::new(grid.cell_count(), 0);

            while gen.step(&mut grid, &mut rng).is_some() {}

            assert!(gen.is_done(), "seed {seed}: not done");
            assert_eq!(
                count_passages(&grid),
                35,
                "seed {seed}: wrong passage count"
            );
            assert!(
                all_cells_reachable(&grid),
                "seed {seed}: not all cells reachable"
            );
        }
    }
}
