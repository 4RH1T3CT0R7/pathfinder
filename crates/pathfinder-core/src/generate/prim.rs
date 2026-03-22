use super::{GenAction, GenStep, SteppableGenerator};
use crate::maze::MazeGrid;
use crate::rng::Xoshiro256;

/// Randomized Prim's algorithm maze generator, implemented as a state machine.
///
/// The algorithm maintains a *frontier* -- the set of walls adjacent to cells
/// already included in the maze. On each step a random wall is removed from the
/// frontier; if the cell on the other side has not been visited, the wall is
/// carved and the new cell's walls are added to the frontier.
///
/// This produces mazes with many short dead ends and a generally "bushy" feel,
/// in contrast to DFS which creates long winding corridors.
pub struct PrimGenerator {
    /// Frontier walls: each entry is (maze_cell, neighbour_cell, direction from maze_cell).
    frontier: Vec<(u32, u32, crate::maze::Direction)>,
    /// Per-cell visited flag.
    visited: Vec<bool>,
    /// Whether generation is complete.
    done: bool,
}

impl PrimGenerator {
    pub fn new(cell_count: u32, start_cell: u32) -> Self {
        let mut visited = vec![false; cell_count as usize];
        visited[start_cell as usize] = true;
        PrimGenerator {
            frontier: Vec::new(),
            visited,
            done: false,
        }
    }

    /// Add all walls of `cell` that border unvisited neighbours to the frontier.
    fn add_frontier_walls(&mut self, cell: u32, maze: &dyn MazeGrid) {
        for (neighbour, dir) in maze.neighbors(cell) {
            if !self.visited[neighbour as usize] {
                self.frontier.push((cell, neighbour, dir));
            }
        }
    }
}

impl SteppableGenerator for PrimGenerator {
    fn step(&mut self, maze: &mut dyn MazeGrid, rng: &mut Xoshiro256) -> Option<GenStep> {
        if self.done {
            return None;
        }

        // On the very first call the frontier is empty -- seed it from the start cell.
        // We detect this by checking whether the frontier is empty while we still have
        // an unvisited cell somewhere (the start cell has been marked visited in `new`).
        if self.frontier.is_empty() {
            // Find the (single) visited cell to seed frontier.
            if let Some(start) = self.visited.iter().position(|&v| v) {
                self.add_frontier_walls(start as u32, maze);
            }
            if self.frontier.is_empty() {
                // 1x1 grid or no neighbours at all.
                self.done = true;
                return None;
            }
        }

        // Pick a random wall from the frontier.
        loop {
            if self.frontier.is_empty() {
                self.done = true;
                return None;
            }

            let idx = rng.next_bound(self.frontier.len() as u32) as usize;
            let (maze_cell, neighbour, dir) = self.frontier.swap_remove(idx);

            if self.visited[neighbour as usize] {
                // Both sides already in the maze -- discard and try next.
                continue;
            }

            // Carve the wall and mark the neighbour as visited.
            maze.remove_wall(maze_cell, dir);
            self.visited[neighbour as usize] = true;

            // Add the new cell's frontier walls.
            self.add_frontier_walls(neighbour, maze);

            return Some(GenStep {
                cell: neighbour,
                action: GenAction::RemoveWall(dir),
            });
        }
    }

    fn reset(&mut self, start_cell: u32) {
        self.visited.fill(false);
        self.visited[start_cell as usize] = true;
        self.frontier.clear();
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
    fn test_prim_generates_perfect_maze() {
        let mut grid = RectGrid::new(10, 10);
        let mut rng = Xoshiro256::new(42);
        let mut gen = PrimGenerator::new(grid.cell_count(), 0);

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
    fn test_prim_deterministic() {
        let mut grid1 = RectGrid::new(5, 5);
        let mut grid2 = RectGrid::new(5, 5);
        let mut rng1 = Xoshiro256::new(123);
        let mut rng2 = Xoshiro256::new(123);
        let mut gen1 = PrimGenerator::new(25, 0);
        let mut gen2 = PrimGenerator::new(25, 0);

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
    fn test_prim_all_cells_reachable() {
        let mut grid = RectGrid::new(8, 8);
        let mut rng = Xoshiro256::new(77);
        let mut gen = PrimGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}

        assert!(all_cells_reachable(&grid), "Not all cells are reachable");
    }

    #[test]
    fn test_prim_small_grid() {
        let mut grid = RectGrid::new(2, 2);
        let mut rng = Xoshiro256::new(1);
        let mut gen = PrimGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}

        assert!(gen.is_done());
        assert_eq!(count_passages(&grid), 3); // 2*2 - 1
        assert!(all_cells_reachable(&grid));
    }

    #[test]
    fn test_prim_rectangular_grid() {
        let mut grid = RectGrid::new(7, 3);
        let mut rng = Xoshiro256::new(999);
        let mut gen = PrimGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}

        assert!(gen.is_done());
        assert_eq!(count_passages(&grid), 20); // 7*3 - 1
        assert!(all_cells_reachable(&grid));
    }

    #[test]
    fn test_prim_start_from_center() {
        let mut grid = RectGrid::new(6, 6);
        let start = grid.cell_index(3, 3);
        let mut rng = Xoshiro256::new(55);
        let mut gen = PrimGenerator::new(grid.cell_count(), start);

        while gen.step(&mut grid, &mut rng).is_some() {}

        assert!(gen.is_done());
        assert_eq!(count_passages(&grid), 35); // 6*6 - 1
        assert!(all_cells_reachable(&grid));
    }

    #[test]
    fn test_prim_reset() {
        let mut grid = RectGrid::new(5, 5);
        let mut rng = Xoshiro256::new(42);
        let mut gen = PrimGenerator::new(grid.cell_count(), 0);

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
}
