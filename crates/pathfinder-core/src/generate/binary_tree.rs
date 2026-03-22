use super::{GenAction, GenStep, SteppableGenerator};
use crate::maze::{Direction, MazeGrid};
use crate::rng::Xoshiro256;

/// Binary Tree maze generator.
///
/// The simplest possible maze generation algorithm. For each cell, it randomly
/// carves a passage either North or West. At the borders where only one
/// direction is available, it carves in that direction. The top-left corner
/// cell (row 0, col 0) has neither North nor West neighbors, so it is simply
/// skipped.
///
/// Properties:
/// - **Bias**: always produces a clear corridor along the entire North row
///   and the entire West column.
/// - **Perfect maze**: every cell is reachable (spanning tree with N*M - 1
///   passages).
/// - **Speed**: O(N*M) with exactly one decision per cell.
/// - **Memory**: O(1) extra state (just a cursor).
///
/// Cells are processed in row-major order (row 0 left-to-right, then row 1,
/// etc.). Each `step()` call processes one cell.
pub struct BinaryTreeGenerator {
    cell_count: u32,
    /// The next cell to process (row-major index).
    cursor: u32,
    done: bool,
}

impl BinaryTreeGenerator {
    pub fn new(cell_count: u32, _start_cell: u32) -> Self {
        BinaryTreeGenerator {
            cell_count,
            cursor: 0,
            done: false,
        }
    }
}

impl SteppableGenerator for BinaryTreeGenerator {
    fn step(&mut self, maze: &mut dyn MazeGrid, rng: &mut Xoshiro256) -> Option<GenStep> {
        if self.done {
            return None;
        }

        if self.cursor >= self.cell_count {
            self.done = true;
            return None;
        }

        let cell = self.cursor;
        self.cursor += 1;

        let (col, row) = maze.cell_coords(cell);
        let can_go_north = row > 0;
        let can_go_west = col > 0;

        let action = match (can_go_north, can_go_west) {
            (false, false) => GenAction::Visit,
            (true, false) => {
                maze.remove_wall(cell, Direction::NORTH);
                GenAction::RemoveWall(Direction::NORTH)
            }
            (false, true) => {
                maze.remove_wall(cell, Direction::WEST);
                GenAction::RemoveWall(Direction::WEST)
            }
            (true, true) => {
                let dir = if rng.next_bound(2) == 0 {
                    Direction::NORTH
                } else {
                    Direction::WEST
                };
                maze.remove_wall(cell, dir);
                GenAction::RemoveWall(dir)
            }
        };

        Some(GenStep { cell, action })
    }

    fn reset(&mut self, _start_cell: u32) {
        self.cursor = 0;
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
    use crate::rng::Xoshiro256;

    /// Count open passages (edges with walls removed) avoiding double-counting.
    fn count_passages(grid: &RectGrid) -> u32 {
        let mut open_passages = 0;
        for cell in 0..grid.cell_count() {
            let (col, row) = grid.cell_coords(cell);
            if col < grid.width() - 1 && !grid.has_wall(cell, Direction::EAST) {
                open_passages += 1;
            }
            if row < grid.height() - 1 && !grid.has_wall(cell, Direction::SOUTH) {
                open_passages += 1;
            }
        }
        open_passages
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
    fn test_binary_tree_generates_perfect_maze() {
        let mut grid = RectGrid::new(10, 10);
        let mut rng = Xoshiro256::new(42);
        let mut gen = BinaryTreeGenerator::new(grid.cell_count(), 0);

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
    fn test_binary_tree_deterministic() {
        let mut grid1 = RectGrid::new(5, 5);
        let mut grid2 = RectGrid::new(5, 5);
        let mut rng1 = Xoshiro256::new(123);
        let mut rng2 = Xoshiro256::new(123);
        let mut gen1 = BinaryTreeGenerator::new(25, 0);
        let mut gen2 = BinaryTreeGenerator::new(25, 0);

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
    fn test_binary_tree_all_cells_reachable() {
        let mut grid = RectGrid::new(8, 8);
        let mut rng = Xoshiro256::new(77);
        let mut gen = BinaryTreeGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}

        assert!(all_cells_reachable(&grid), "Not all cells are reachable");
    }

    #[test]
    fn test_binary_tree_north_row_clear() {
        // The entire North row should have a clear corridor going West (all
        // cells in row 0 except cell (0,0) should have their West wall
        // removed).
        let mut grid = RectGrid::new(10, 10);
        let mut rng = Xoshiro256::new(42);
        let mut gen = BinaryTreeGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}

        for col in 1..grid.width() {
            let cell = grid.cell_index(col, 0);
            assert!(
                !grid.has_wall(cell, Direction::WEST),
                "Cell ({col}, 0) should have West wall removed"
            );
        }
    }

    #[test]
    fn test_binary_tree_west_column_clear() {
        // The entire West column should have a clear corridor going North (all
        // cells in col 0 except cell (0,0) should have their North wall
        // removed).
        let mut grid = RectGrid::new(10, 10);
        let mut rng = Xoshiro256::new(42);
        let mut gen = BinaryTreeGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}

        for row in 1..grid.height() {
            let cell = grid.cell_index(0, row);
            assert!(
                !grid.has_wall(cell, Direction::NORTH),
                "Cell (0, {row}) should have North wall removed"
            );
        }
    }

    #[test]
    fn test_binary_tree_step_count() {
        // Binary tree processes every cell exactly once, so the number of
        // steps equals the number of cells.
        let mut grid = RectGrid::new(6, 7);
        let mut rng = Xoshiro256::new(42);
        let mut gen = BinaryTreeGenerator::new(grid.cell_count(), 0);

        let mut steps = 0;
        while gen.step(&mut grid, &mut rng).is_some() {
            steps += 1;
        }
        assert_eq!(steps, 42); // 6 * 7 = 42
    }

    #[test]
    fn test_binary_tree_reset() {
        let mut grid = RectGrid::new(5, 5);
        let mut rng = Xoshiro256::new(42);
        let mut gen = BinaryTreeGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}
        assert!(gen.is_done());

        gen.reset(0);
        assert!(!gen.is_done());

        let mut grid2 = RectGrid::new(5, 5);
        let mut steps = 0;
        while gen.step(&mut grid2, &mut rng).is_some() {
            steps += 1;
        }
        assert!(gen.is_done());
        assert!(steps > 0);
    }

    #[test]
    fn test_binary_tree_small_grid() {
        let mut grid = RectGrid::new(2, 2);
        let mut rng = Xoshiro256::new(99);
        let mut gen = BinaryTreeGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}
        assert_eq!(count_passages(&grid), 3);
    }

    #[test]
    fn test_binary_tree_single_cell() {
        let mut grid = RectGrid::new(1, 1);
        let mut rng = Xoshiro256::new(1);
        let mut gen = BinaryTreeGenerator::new(grid.cell_count(), 0);

        // The only cell is (0,0) -- top-left corner, so it is visited but no
        // wall is carved.
        let step = gen.step(&mut grid, &mut rng);
        assert!(step.is_some());
        let step = step.unwrap();
        assert_eq!(step.cell, 0);
        assert!(matches!(step.action, GenAction::Visit));

        let step2 = gen.step(&mut grid, &mut rng);
        assert!(step2.is_none());
        assert!(gen.is_done());
    }
}
