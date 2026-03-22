use super::{GenAction, GenStep, SteppableGenerator};
use crate::maze::MazeGrid;
use crate::rng::Xoshiro256;

/// Wilson's Algorithm maze generator (Loop-Erased Random Walk).
///
/// Produces uniform spanning trees -- every possible perfect maze is equally
/// likely. The algorithm works as follows:
///
/// 1. Mark one cell as part of the maze (visited).
/// 2. Pick a random unvisited cell and perform a random walk until a visited
///    cell is reached.
/// 3. Erase any loops from the walk, then carve the resulting path into the
///    maze, marking all cells on the path as visited.
/// 4. Repeat from step 2 until every cell is visited.
///
/// The state machine alternates between two phases:
/// - **RandomWalk**: take one random walk step per `step()` call.
/// - **CarvePath**: carve one edge of the loop-erased path per `step()` call.
pub struct WilsonGenerator {
    cell_count: u32,
    /// Whether each cell is part of the maze (visited / carved).
    in_maze: Vec<bool>,
    /// During a random walk, stores the direction taken *from* each cell. A
    /// value of `u8::MAX` means the cell has not been visited during the
    /// current walk.
    walk_dir: Vec<u8>,
    /// Current phase of the state machine.
    phase: WilsonPhase,
    /// The cell where the current random walk started.
    walk_start: u32,
    /// The cell the random walker is currently at.
    walk_head: u32,
    /// When carving, the cell we are about to carve from.
    carve_cursor: u32,
    /// Generation finished.
    done: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WilsonPhase {
    /// Need to pick a new unvisited cell to start a walk from.
    PickStart,
    /// Performing the random walk.
    RandomWalk,
    /// Carving the loop-erased path into the maze.
    CarvePath,
}

/// Encodes a `Direction` as a `u8` index for compact storage in `walk_dir`.
fn dir_to_index(dir: crate::maze::Direction) -> u8 {
    dir.0
}

/// Decodes a `u8` index back to a `Direction`.
fn index_to_dir(idx: u8) -> crate::maze::Direction {
    crate::maze::Direction(idx)
}

const NO_DIR: u8 = u8::MAX;

impl WilsonGenerator {
    pub fn new(cell_count: u32, start_cell: u32) -> Self {
        let mut in_maze = vec![false; cell_count as usize];
        in_maze[start_cell as usize] = true;

        WilsonGenerator {
            cell_count,
            in_maze,
            walk_dir: vec![NO_DIR; cell_count as usize],
            phase: WilsonPhase::PickStart,
            walk_start: 0,
            walk_head: 0,
            carve_cursor: 0,
            done: false,
        }
    }

    /// Find the first unvisited cell starting from a given hint index. Returns
    /// `None` if all cells are in the maze.
    fn find_unvisited(&self, rng: &mut Xoshiro256) -> Option<u32> {
        // Pick a random starting point for the scan to avoid bias toward low
        // indices while keeping O(n) worst case.
        let offset = rng.next_bound(self.cell_count);
        for i in 0..self.cell_count {
            let idx = (offset + i) % self.cell_count;
            if !self.in_maze[idx as usize] {
                return Some(idx);
            }
        }
        None
    }

    /// Clear `walk_dir` entries that were written during the last random walk.
    /// We do a full clear -- the vector is small (one byte per cell).
    fn clear_walk(&mut self) {
        self.walk_dir.fill(NO_DIR);
    }
}

impl SteppableGenerator for WilsonGenerator {
    fn step(&mut self, maze: &mut dyn MazeGrid, rng: &mut Xoshiro256) -> Option<GenStep> {
        if self.done {
            return None;
        }

        match self.phase {
            WilsonPhase::PickStart => {
                match self.find_unvisited(rng) {
                    Some(cell) => {
                        self.clear_walk();
                        self.walk_start = cell;
                        self.walk_head = cell;
                        self.phase = WilsonPhase::RandomWalk;
                        Some(GenStep {
                            cell,
                            action: GenAction::Visit,
                        })
                    }
                    None => {
                        // All cells in maze -- we are done.
                        self.done = true;
                        None
                    }
                }
            }

            WilsonPhase::RandomWalk => {
                let current = self.walk_head;
                let neighbors = maze.neighbors(current);

                // Pick a random neighbor.
                let idx = rng.next_bound(neighbors.len() as u32) as usize;
                let (next_cell, dir) = neighbors[idx];

                // Record the direction we left `current` through.
                self.walk_dir[current as usize] = dir_to_index(dir);

                if self.in_maze[next_cell as usize] {
                    // Walk hit the maze -- start carving the loop-erased path.
                    self.carve_cursor = self.walk_start;
                    self.phase = WilsonPhase::CarvePath;
                    // Report the walk arriving at a visited cell.
                    Some(GenStep {
                        cell: next_cell,
                        action: GenAction::Visit,
                    })
                } else {
                    // Continue walking.
                    self.walk_head = next_cell;
                    Some(GenStep {
                        cell: next_cell,
                        action: GenAction::Visit,
                    })
                }
            }

            WilsonPhase::CarvePath => {
                let cell = self.carve_cursor;
                let dir_idx = self.walk_dir[cell as usize];

                if dir_idx == NO_DIR || self.in_maze[cell as usize] {
                    // We have reached the maze boundary of the walk -- done
                    // carving this path. Go pick a new start.
                    self.phase = WilsonPhase::PickStart;
                    // Recurse into PickStart to emit the next meaningful step.
                    return self.step(maze, rng);
                }

                let dir = index_to_dir(dir_idx);

                // Find the neighbor in the recorded direction.
                let neighbors = maze.neighbors(cell);
                let (next_cell, _) = neighbors
                    .iter()
                    .find(|&&(_, d)| d == dir)
                    .copied()
                    .expect("walk_dir points to a valid neighbor");

                // Carve the passage.
                maze.remove_wall(cell, dir);
                self.in_maze[cell as usize] = true;

                // Advance the cursor.
                self.carve_cursor = next_cell;

                // If the next cell is already in the maze, we are done with
                // this path.
                if self.in_maze[next_cell as usize] {
                    self.phase = WilsonPhase::PickStart;
                }

                Some(GenStep {
                    cell,
                    action: GenAction::RemoveWall(dir),
                })
            }
        }
    }

    fn reset(&mut self, start_cell: u32) {
        self.in_maze.fill(false);
        self.in_maze[start_cell as usize] = true;
        self.walk_dir.fill(NO_DIR);
        self.phase = WilsonPhase::PickStart;
        self.walk_start = 0;
        self.walk_head = 0;
        self.carve_cursor = 0;
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

    #[test]
    fn test_wilson_generates_perfect_maze() {
        let mut grid = RectGrid::new(10, 10);
        let mut rng = Xoshiro256::new(42);
        let mut gen = WilsonGenerator::new(grid.cell_count(), 0);

        let mut steps = 0;
        while let Some(_step) = gen.step(&mut grid, &mut rng) {
            steps += 1;
        }
        assert!(gen.is_done());
        assert!(steps > 0);

        // A perfect maze on a 10x10 grid has exactly 99 removed walls.
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
        assert_eq!(open_passages, 99);
    }

    #[test]
    fn test_wilson_deterministic() {
        let mut grid1 = RectGrid::new(5, 5);
        let mut grid2 = RectGrid::new(5, 5);
        let mut rng1 = Xoshiro256::new(123);
        let mut rng2 = Xoshiro256::new(123);
        let mut gen1 = WilsonGenerator::new(25, 0);
        let mut gen2 = WilsonGenerator::new(25, 0);

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
    fn test_wilson_all_cells_reachable() {
        let mut grid = RectGrid::new(8, 8);
        let mut rng = Xoshiro256::new(77);
        let mut gen = WilsonGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}

        let mut visited = [false; 64];
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

        assert!(visited.iter().all(|&v| v), "Not all cells are reachable");
    }

    #[test]
    fn test_wilson_reset() {
        let mut grid = RectGrid::new(5, 5);
        let mut rng = Xoshiro256::new(42);
        let mut gen = WilsonGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}
        assert!(gen.is_done());

        // Reset and generate again on a fresh grid.
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
    fn test_wilson_small_grid() {
        // 2x2 grid: should produce exactly 3 passages.
        let mut grid = RectGrid::new(2, 2);
        let mut rng = Xoshiro256::new(99);
        let mut gen = WilsonGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}

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
        assert_eq!(open_passages, 3);
    }

    #[test]
    fn test_wilson_single_cell() {
        // 1x1 grid: nothing to generate.
        let mut grid = RectGrid::new(1, 1);
        let mut rng = Xoshiro256::new(1);
        let mut gen = WilsonGenerator::new(grid.cell_count(), 0);

        let step = gen.step(&mut grid, &mut rng);
        assert!(step.is_none());
        assert!(gen.is_done());
    }
}
