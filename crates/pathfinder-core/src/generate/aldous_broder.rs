use super::{GenAction, GenStep, SteppableGenerator};
use crate::maze::MazeGrid;
use crate::rng::Xoshiro256;

/// Aldous-Broder maze generator, implemented as a state machine.
///
/// Performs a random walk on the grid. Starting at a given cell, each step moves
/// to a uniformly random neighbor. If that neighbor has not been visited yet, the
/// wall between the current cell and the neighbor is carved, connecting it to the
/// spanning tree. The algorithm terminates when every cell has been visited.
///
/// This produces a uniform spanning tree (every possible perfect maze is equally
/// likely), but convergence is slow on average because revisits of already-visited
/// cells are wasted work.
pub struct AldousBroderGenerator {
    current: u32,
    visited: Vec<bool>,
    visited_count: u32,
    total_cells: u32,
    done: bool,
}

impl AldousBroderGenerator {
    pub fn new(cell_count: u32, start_cell: u32) -> Self {
        let mut visited = vec![false; cell_count as usize];
        visited[start_cell as usize] = true;
        AldousBroderGenerator {
            current: start_cell,
            visited,
            visited_count: 1,
            total_cells: cell_count,
            done: cell_count <= 1,
        }
    }
}

impl SteppableGenerator for AldousBroderGenerator {
    fn step(&mut self, maze: &mut dyn MazeGrid, rng: &mut Xoshiro256) -> Option<GenStep> {
        if self.done {
            return None;
        }

        // Get all neighbors of the current cell and pick one at random.
        let neighbors = maze.neighbors(self.current);
        let idx = rng.next_bound(neighbors.len() as u32) as usize;
        let (neighbor, dir) = neighbors[idx];

        let step = if !self.visited[neighbor as usize] {
            // First visit: carve the wall and mark visited.
            self.visited[neighbor as usize] = true;
            self.visited_count += 1;
            maze.remove_wall(self.current, dir);

            if self.visited_count == self.total_cells {
                self.done = true;
            }

            GenStep {
                cell: neighbor,
                action: GenAction::RemoveWall(dir),
            }
        } else {
            // Already visited: just move there (no carving).
            GenStep {
                cell: neighbor,
                action: GenAction::Visit,
            }
        };

        self.current = neighbor;
        Some(step)
    }

    fn reset(&mut self, start_cell: u32) {
        self.visited.fill(false);
        self.visited[start_cell as usize] = true;
        self.current = start_cell;
        self.visited_count = 1;
        self.done = self.total_cells <= 1;
    }

    fn is_done(&self) -> bool {
        self.done
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maze::{Direction, RectGrid};
    use crate::rng::Xoshiro256;

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
    fn test_aldous_broder_generates_perfect_maze() {
        let mut grid = RectGrid::new(10, 10);
        let mut rng = Xoshiro256::new(42);
        let mut gen = AldousBroderGenerator::new(grid.cell_count(), 0);

        let mut steps = 0;
        while let Some(_step) = gen.step(&mut grid, &mut rng) {
            steps += 1;
        }
        assert!(gen.is_done());
        assert!(steps >= 99); // At least N*M - 1 steps (one per carve).

        // A perfect maze on a 10x10 grid has exactly 99 passages.
        assert_eq!(count_passages(&grid), 99);
    }

    #[test]
    fn test_aldous_broder_deterministic() {
        let mut grid1 = RectGrid::new(5, 5);
        let mut grid2 = RectGrid::new(5, 5);
        let mut rng1 = Xoshiro256::new(123);
        let mut rng2 = Xoshiro256::new(123);
        let mut gen1 = AldousBroderGenerator::new(25, 0);
        let mut gen2 = AldousBroderGenerator::new(25, 0);

        while let (Some(s1), Some(s2)) = (
            gen1.step(&mut grid1, &mut rng1),
            gen2.step(&mut grid2, &mut rng2),
        ) {
            assert_eq!(s1.cell, s2.cell);
        }

        // Verify both grids are identical.
        for cell in 0..25 {
            assert_eq!(grid1.wall_bits(cell), grid2.wall_bits(cell));
        }
    }

    #[test]
    fn test_aldous_broder_all_cells_reachable() {
        let mut grid = RectGrid::new(8, 8);
        let mut rng = Xoshiro256::new(77);
        let mut gen = AldousBroderGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}

        assert_eq!(bfs_reachable(&grid), 64, "Not all cells are reachable");
    }

    #[test]
    fn test_aldous_broder_various_sizes() {
        for (w, h) in [(1, 1), (2, 2), (3, 7), (10, 10)] {
            let mut grid = RectGrid::new(w, h);
            let mut rng = Xoshiro256::new(w as u64 * 100 + h as u64);
            let mut gen = AldousBroderGenerator::new(grid.cell_count(), 0);

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
    fn test_aldous_broder_single_cell() {
        let mut grid = RectGrid::new(1, 1);
        let mut rng = Xoshiro256::new(42);
        let mut gen = AldousBroderGenerator::new(1, 0);

        // Should immediately be done (single cell, nothing to carve).
        assert!(gen.is_done());
        assert!(gen.step(&mut grid, &mut rng).is_none());
    }

    #[test]
    fn test_aldous_broder_reset() {
        let mut grid = RectGrid::new(5, 5);
        let mut rng = Xoshiro256::new(42);
        let mut gen = AldousBroderGenerator::new(grid.cell_count(), 0);

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

    #[test]
    fn test_aldous_broder_many_revisits() {
        // Aldous-Broder typically takes many more steps than cells because of revisits.
        let mut grid = RectGrid::new(6, 6);
        let mut rng = Xoshiro256::new(55);
        let mut gen = AldousBroderGenerator::new(grid.cell_count(), 0);

        let mut total_steps = 0u32;
        let mut carve_steps = 0u32;
        while let Some(step) = gen.step(&mut grid, &mut rng) {
            total_steps += 1;
            if matches!(step.action, GenAction::RemoveWall(_)) {
                carve_steps += 1;
            }
        }

        // Exactly N*M - 1 carves.
        assert_eq!(carve_steps, 35);
        // Total steps should be significantly more than carve steps.
        assert!(
            total_steps > carve_steps,
            "Expected revisits: total_steps={total_steps}, carve_steps={carve_steps}"
        );
    }
}
