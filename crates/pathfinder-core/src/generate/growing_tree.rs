use super::{GenAction, GenStep, SteppableGenerator};
use crate::maze::MazeGrid;
use crate::rng::Xoshiro256;

/// Cell selection strategy for the Growing Tree algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionStrategy {
    /// Always pick the newest (last added) cell. Equivalent to recursive
    /// backtracker / DFS. Produces long winding corridors.
    Newest,
    /// Pick a cell uniformly at random. Similar to Prim's algorithm. Produces
    /// shorter, more branching passages.
    Random,
    /// Pick the oldest (first added) cell. Produces a different texture than
    /// Newest or Random.
    Oldest,
}

/// Growing Tree maze generator.
///
/// Generalizes several algorithms depending on the cell selection strategy:
/// - `Newest` selection behaves like a recursive backtracker (DFS).
/// - `Random` selection behaves like a randomized Prim's algorithm.
/// - `Oldest` selection produces a breadth-first flavored maze.
///
/// Algorithm:
/// 1. Add the start cell to the active list and mark it visited.
/// 2. Select a cell from the active list using the chosen strategy.
/// 3. If the selected cell has unvisited neighbors, pick one at random, carve a
///    passage between them, mark the neighbor visited, and add it to the list.
/// 4. If the selected cell has no unvisited neighbors, remove it from the list.
/// 5. When the list is empty, the maze is complete.
pub struct GrowingTreeGenerator {
    active: Vec<u32>,
    visited: Vec<bool>,
    strategy: SelectionStrategy,
    done: bool,
}

impl GrowingTreeGenerator {
    /// Create a new generator with the given strategy.
    pub fn with_strategy(cell_count: u32, start_cell: u32, strategy: SelectionStrategy) -> Self {
        let mut visited = vec![false; cell_count as usize];
        visited[start_cell as usize] = true;

        GrowingTreeGenerator {
            active: vec![start_cell],
            visited,
            strategy,
            done: false,
        }
    }

    /// Create a new generator with the default `Newest` strategy (equivalent to
    /// recursive backtracker).
    pub fn new(cell_count: u32, start_cell: u32) -> Self {
        Self::with_strategy(cell_count, start_cell, SelectionStrategy::Newest)
    }

    /// Select an index into `self.active` based on the current strategy.
    fn select_index(&self, rng: &mut Xoshiro256) -> usize {
        match self.strategy {
            SelectionStrategy::Newest => self.active.len() - 1,
            SelectionStrategy::Oldest => 0,
            SelectionStrategy::Random => rng.next_bound(self.active.len() as u32) as usize,
        }
    }
}

impl SteppableGenerator for GrowingTreeGenerator {
    fn step(&mut self, maze: &mut dyn MazeGrid, rng: &mut Xoshiro256) -> Option<GenStep> {
        if self.done || self.active.is_empty() {
            self.done = true;
            return None;
        }

        let idx = self.select_index(rng);
        let current = self.active[idx];

        // Collect unvisited neighbors of the selected cell.
        let neighbors = maze.neighbors(current);
        let unvisited: Vec<(u32, crate::maze::Direction)> = neighbors
            .into_iter()
            .filter(|&(n, _)| !self.visited[n as usize])
            .collect();

        if unvisited.is_empty() {
            // No unvisited neighbors -- remove this cell from the active list.
            self.active.swap_remove(idx);
            if self.active.is_empty() {
                self.done = true;
            }
            return Some(GenStep {
                cell: current,
                action: GenAction::Backtrack,
            });
        }

        // Pick a random unvisited neighbor.
        let n_idx = rng.next_bound(unvisited.len() as u32) as usize;
        let (neighbor, dir) = unvisited[n_idx];

        // Carve a passage.
        maze.remove_wall(current, dir);
        self.visited[neighbor as usize] = true;
        self.active.push(neighbor);

        Some(GenStep {
            cell: neighbor,
            action: GenAction::RemoveWall(dir),
        })
    }

    fn reset(&mut self, start_cell: u32) {
        self.visited.fill(false);
        self.visited[start_cell as usize] = true;
        self.active.clear();
        self.active.push(start_cell);
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
            if col < grid.width() - 1 && !grid.has_wall(cell, crate::maze::Direction::EAST) {
                open_passages += 1;
            }
            if row < grid.height() - 1 && !grid.has_wall(cell, crate::maze::Direction::SOUTH) {
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

    // ---- Tests for Newest (default) strategy ----

    #[test]
    fn test_growing_tree_newest_perfect_maze() {
        let mut grid = RectGrid::new(10, 10);
        let mut rng = Xoshiro256::new(42);
        let mut gen = GrowingTreeGenerator::new(grid.cell_count(), 0);

        let mut steps = 0;
        while let Some(_step) = gen.step(&mut grid, &mut rng) {
            steps += 1;
        }
        assert!(gen.is_done());
        assert!(steps > 0);
        assert_eq!(count_passages(&grid), 99);
    }

    #[test]
    fn test_growing_tree_newest_deterministic() {
        let mut grid1 = RectGrid::new(5, 5);
        let mut grid2 = RectGrid::new(5, 5);
        let mut rng1 = Xoshiro256::new(123);
        let mut rng2 = Xoshiro256::new(123);
        let mut gen1 = GrowingTreeGenerator::new(25, 0);
        let mut gen2 = GrowingTreeGenerator::new(25, 0);

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
    fn test_growing_tree_newest_all_cells_reachable() {
        let mut grid = RectGrid::new(8, 8);
        let mut rng = Xoshiro256::new(77);
        let mut gen = GrowingTreeGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}

        assert!(all_cells_reachable(&grid), "Not all cells are reachable");
    }

    // ---- Tests for Random strategy ----

    #[test]
    fn test_growing_tree_random_perfect_maze() {
        let mut grid = RectGrid::new(10, 10);
        let mut rng = Xoshiro256::new(42);
        let mut gen =
            GrowingTreeGenerator::with_strategy(grid.cell_count(), 0, SelectionStrategy::Random);

        while gen.step(&mut grid, &mut rng).is_some() {}
        assert!(gen.is_done());
        assert_eq!(count_passages(&grid), 99);
    }

    #[test]
    fn test_growing_tree_random_all_cells_reachable() {
        let mut grid = RectGrid::new(8, 8);
        let mut rng = Xoshiro256::new(77);
        let mut gen =
            GrowingTreeGenerator::with_strategy(grid.cell_count(), 0, SelectionStrategy::Random);

        while gen.step(&mut grid, &mut rng).is_some() {}

        assert!(all_cells_reachable(&grid), "Not all cells are reachable");
    }

    #[test]
    fn test_growing_tree_random_deterministic() {
        let mut grid1 = RectGrid::new(5, 5);
        let mut grid2 = RectGrid::new(5, 5);
        let mut rng1 = Xoshiro256::new(456);
        let mut rng2 = Xoshiro256::new(456);
        let mut gen1 =
            GrowingTreeGenerator::with_strategy(25, 0, SelectionStrategy::Random);
        let mut gen2 =
            GrowingTreeGenerator::with_strategy(25, 0, SelectionStrategy::Random);

        while let (Some(s1), Some(s2)) =
            (gen1.step(&mut grid1, &mut rng1), gen2.step(&mut grid2, &mut rng2))
        {
            assert_eq!(s1.cell, s2.cell);
        }

        for cell in 0..25 {
            assert_eq!(grid1.wall_bits(cell), grid2.wall_bits(cell));
        }
    }

    // ---- Tests for Oldest strategy ----

    #[test]
    fn test_growing_tree_oldest_perfect_maze() {
        let mut grid = RectGrid::new(10, 10);
        let mut rng = Xoshiro256::new(42);
        let mut gen =
            GrowingTreeGenerator::with_strategy(grid.cell_count(), 0, SelectionStrategy::Oldest);

        while gen.step(&mut grid, &mut rng).is_some() {}
        assert!(gen.is_done());
        assert_eq!(count_passages(&grid), 99);
    }

    #[test]
    fn test_growing_tree_oldest_all_cells_reachable() {
        let mut grid = RectGrid::new(8, 8);
        let mut rng = Xoshiro256::new(77);
        let mut gen =
            GrowingTreeGenerator::with_strategy(grid.cell_count(), 0, SelectionStrategy::Oldest);

        while gen.step(&mut grid, &mut rng).is_some() {}

        assert!(all_cells_reachable(&grid), "Not all cells are reachable");
    }

    // ---- General tests ----

    #[test]
    fn test_growing_tree_reset() {
        let mut grid = RectGrid::new(5, 5);
        let mut rng = Xoshiro256::new(42);
        let mut gen = GrowingTreeGenerator::new(grid.cell_count(), 0);

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
    fn test_growing_tree_small_grid() {
        let mut grid = RectGrid::new(2, 2);
        let mut rng = Xoshiro256::new(99);
        let mut gen = GrowingTreeGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}
        assert_eq!(count_passages(&grid), 3);
    }

    #[test]
    fn test_growing_tree_single_cell() {
        let mut grid = RectGrid::new(1, 1);
        let mut rng = Xoshiro256::new(1);
        let mut gen = GrowingTreeGenerator::new(grid.cell_count(), 0);

        // The active list starts with one cell, but it has no neighbors, so it
        // gets removed immediately and the generator finishes.
        let step = gen.step(&mut grid, &mut rng);
        // First step should be a Backtrack (removing the only cell).
        assert!(step.is_some());
        let step2 = gen.step(&mut grid, &mut rng);
        assert!(step2.is_none());
        assert!(gen.is_done());
    }
}
