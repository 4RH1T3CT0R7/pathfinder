use super::{GenAction, GenStep, SteppableGenerator};
use crate::maze::MazeGrid;
use crate::rng::Xoshiro256;

/// Hunt-and-Kill maze generator, implemented as a state machine.
///
/// Operates in two alternating phases:
///
/// **Walk phase**: From the current cell, pick a random unvisited neighbor and carve
/// the wall to it. Continue walking until the current cell has no unvisited neighbors.
///
/// **Hunt phase**: Scan the grid row by row for an unvisited cell that is adjacent to
/// at least one visited cell. Carve the wall between them and switch back to the walk
/// phase from that cell.
///
/// The algorithm terminates when the hunt phase finds no unvisited cells with visited
/// neighbors (i.e., every cell has been visited).
///
/// Compared to DFS, Hunt-and-Kill does not use a stack for backtracking. Instead, it
/// performs a linear scan ("hunt") when stuck. This produces mazes with longer passages
/// and fewer dead-ends than DFS on average.
pub struct HuntAndKillGenerator {
    current: u32,
    visited: Vec<bool>,
    visited_count: u32,
    total_cells: u32,
    /// Next cell index to start scanning from during the hunt phase.
    hunt_scan_pos: u32,
    phase: Phase,
    done: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Walk,
    Hunt,
}

impl HuntAndKillGenerator {
    pub fn new(cell_count: u32, start_cell: u32) -> Self {
        let mut visited = vec![false; cell_count as usize];
        visited[start_cell as usize] = true;
        HuntAndKillGenerator {
            current: start_cell,
            visited,
            visited_count: 1,
            total_cells: cell_count,
            hunt_scan_pos: 0,
            phase: Phase::Walk,
            done: cell_count <= 1,
        }
    }
}

impl SteppableGenerator for HuntAndKillGenerator {
    fn step(&mut self, maze: &mut dyn MazeGrid, rng: &mut Xoshiro256) -> Option<GenStep> {
        if self.done {
            return None;
        }

        match self.phase {
            Phase::Walk => {
                // Collect unvisited neighbors of the current cell.
                let neighbors = maze.neighbors(self.current);
                let mut unvisited: Vec<(u32, crate::maze::Direction)> = neighbors
                    .into_iter()
                    .filter(|&(n, _)| !self.visited[n as usize])
                    .collect();

                if unvisited.is_empty() {
                    // Stuck: switch to hunt phase.
                    // Don't reset hunt_scan_pos — resume from where previous hunt left off,
                    // since earlier cells are already visited. This makes hunt O(n) total
                    // instead of O(n^2).
                    self.phase = Phase::Hunt;
                    return self.step(maze, rng);
                }

                // Pick a random unvisited neighbor and carve.
                let idx = rng.next_bound(unvisited.len() as u32) as usize;
                let (neighbor, dir) = unvisited.swap_remove(idx);

                maze.remove_wall(self.current, dir);
                self.visited[neighbor as usize] = true;
                self.visited_count += 1;
                self.current = neighbor;

                if self.visited_count == self.total_cells {
                    self.done = true;
                }

                Some(GenStep {
                    cell: neighbor,
                    action: GenAction::RemoveWall(dir),
                })
            }
            Phase::Hunt => {
                // Scan for an unvisited cell adjacent to a visited cell.
                // Start from hunt_scan_pos, wrapping around to cover the full grid.
                for i in 0..self.total_cells {
                    let cell = (self.hunt_scan_pos + i) % self.total_cells;

                    if self.visited[cell as usize] {
                        continue;
                    }

                    // Check if this unvisited cell has any visited neighbors.
                    let neighbors = maze.neighbors(cell);
                    let mut visited_neighbors: Vec<(u32, crate::maze::Direction)> = neighbors
                        .into_iter()
                        .filter(|&(n, _)| self.visited[n as usize])
                        .collect();

                    if visited_neighbors.is_empty() {
                        continue;
                    }

                    // Found a huntable cell. Pick a random visited neighbor to connect to.
                    let idx = rng.next_bound(visited_neighbors.len() as u32) as usize;
                    let (_, dir_to_visited) = visited_neighbors.swap_remove(idx);

                    maze.remove_wall(cell, dir_to_visited);
                    self.visited[cell as usize] = true;
                    self.visited_count += 1;
                    self.current = cell;
                    self.hunt_scan_pos = cell + 1;
                    self.phase = Phase::Walk;

                    if self.visited_count == self.total_cells {
                        self.done = true;
                    }

                    return Some(GenStep {
                        cell,
                        action: GenAction::RemoveWall(dir_to_visited),
                    });
                }

                // Hunt found nothing: all cells visited, generation complete.
                self.done = true;
                None
            }
        }
    }

    fn reset(&mut self, start_cell: u32) {
        self.visited.fill(false);
        self.visited[start_cell as usize] = true;
        self.current = start_cell;
        self.visited_count = 1;
        self.hunt_scan_pos = 0;
        self.phase = Phase::Walk;
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
    fn test_hunt_and_kill_generates_perfect_maze() {
        let mut grid = RectGrid::new(10, 10);
        let mut rng = Xoshiro256::new(42);
        let mut gen = HuntAndKillGenerator::new(grid.cell_count(), 0);

        let mut steps = 0;
        while let Some(_step) = gen.step(&mut grid, &mut rng) {
            steps += 1;
        }
        assert!(gen.is_done());
        assert!(steps > 0);

        // A perfect maze on a 10x10 grid has exactly 99 passages.
        assert_eq!(count_passages(&grid), 99);
    }

    #[test]
    fn test_hunt_and_kill_deterministic() {
        let mut grid1 = RectGrid::new(5, 5);
        let mut grid2 = RectGrid::new(5, 5);
        let mut rng1 = Xoshiro256::new(123);
        let mut rng2 = Xoshiro256::new(123);
        let mut gen1 = HuntAndKillGenerator::new(25, 0);
        let mut gen2 = HuntAndKillGenerator::new(25, 0);

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
    fn test_hunt_and_kill_all_cells_reachable() {
        let mut grid = RectGrid::new(8, 8);
        let mut rng = Xoshiro256::new(77);
        let mut gen = HuntAndKillGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}

        assert_eq!(bfs_reachable(&grid), 64, "Not all cells are reachable");
    }

    #[test]
    fn test_hunt_and_kill_various_sizes() {
        for (w, h) in [(1, 1), (2, 2), (3, 7), (15, 10), (1, 10), (10, 1)] {
            let mut grid = RectGrid::new(w, h);
            let mut rng = Xoshiro256::new(w as u64 * 100 + h as u64);
            let mut gen = HuntAndKillGenerator::new(grid.cell_count(), 0);

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
    fn test_hunt_and_kill_single_cell() {
        let mut grid = RectGrid::new(1, 1);
        let mut rng = Xoshiro256::new(42);
        let mut gen = HuntAndKillGenerator::new(1, 0);

        // Should immediately be done (single cell, nothing to carve).
        assert!(gen.is_done());
        assert!(gen.step(&mut grid, &mut rng).is_none());
    }

    #[test]
    fn test_hunt_and_kill_reset() {
        let mut grid = RectGrid::new(5, 5);
        let mut rng = Xoshiro256::new(42);
        let mut gen = HuntAndKillGenerator::new(grid.cell_count(), 0);

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
    fn test_hunt_and_kill_every_step_carves() {
        // Unlike Aldous-Broder, Hunt-and-Kill never wastes a step: every returned
        // step removes a wall (no pure Visit steps).
        let mut grid = RectGrid::new(6, 6);
        let mut rng = Xoshiro256::new(88);
        let mut gen = HuntAndKillGenerator::new(grid.cell_count(), 0);

        let mut total_steps = 0u32;
        while let Some(step) = gen.step(&mut grid, &mut rng) {
            total_steps += 1;
            assert!(
                matches!(step.action, GenAction::RemoveWall(_)),
                "Hunt-and-Kill should only produce RemoveWall steps, got {:?}",
                step.action
            );
        }

        // Exactly N*M - 1 steps, one per passage.
        assert_eq!(total_steps, 35);
    }

    #[test]
    fn test_hunt_and_kill_starts_from_given_cell() {
        // Start from the bottom-right corner and verify it works.
        let w = 7;
        let h = 7;
        let start = w * h - 1;
        let mut grid = RectGrid::new(w, h);
        let mut rng = Xoshiro256::new(314);
        let mut gen = HuntAndKillGenerator::new(grid.cell_count(), start);

        while gen.step(&mut grid, &mut rng).is_some() {}

        assert!(gen.is_done());
        assert_eq!(count_passages(&grid), w * h - 1);
        assert_eq!(bfs_reachable(&grid), w * h);
    }
}
