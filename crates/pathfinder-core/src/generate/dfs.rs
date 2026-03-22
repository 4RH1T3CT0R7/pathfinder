use super::{GenAction, GenStep, SteppableGenerator};
use crate::maze::{MazeGrid};
use crate::rng::Xoshiro256;

/// DFS Recursive Backtracker maze generator, implemented as a state machine.
pub struct DfsGenerator {
    stack: Vec<u32>,
    visited: Vec<bool>,
    done: bool,
}

impl DfsGenerator {
    pub fn new(cell_count: u32, start_cell: u32) -> Self {
        let mut visited = vec![false; cell_count as usize];
        visited[start_cell as usize] = true;
        DfsGenerator {
            stack: vec![start_cell],
            visited,
            done: false,
        }
    }
}

impl SteppableGenerator for DfsGenerator {
    fn step(&mut self, maze: &mut dyn MazeGrid, rng: &mut Xoshiro256) -> Option<GenStep> {
        if self.done {
            return None;
        }

        let current = match self.stack.last().copied() {
            Some(cell) => cell,
            None => {
                self.done = true;
                return None;
            }
        };

        // Collect unvisited neighbors
        let neighbors = maze.neighbors(current);
        let mut unvisited: Vec<(u32, crate::maze::Direction)> = neighbors
            .into_iter()
            .filter(|&(n, _)| !self.visited[n as usize])
            .collect();

        if unvisited.is_empty() {
            // Backtrack
            self.stack.pop();
            if self.stack.is_empty() {
                self.done = true;
            }
            return Some(GenStep {
                cell: current,
                action: GenAction::Backtrack,
            });
        }

        // Pick a random unvisited neighbor
        let idx = rng.next_bound(unvisited.len() as u32) as usize;
        let (neighbor, dir) = unvisited.swap_remove(idx);

        // Remove wall between current and neighbor
        maze.remove_wall(current, dir);
        self.visited[neighbor as usize] = true;
        self.stack.push(neighbor);

        Some(GenStep {
            cell: neighbor,
            action: GenAction::RemoveWall(dir),
        })
    }

    fn reset(&mut self, start_cell: u32) {
        self.visited.fill(false);
        self.visited[start_cell as usize] = true;
        self.stack.clear();
        self.stack.push(start_cell);
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

    #[test]
    fn test_dfs_generates_perfect_maze() {
        let mut grid = RectGrid::new(10, 10);
        let mut rng = Xoshiro256::new(42);
        let mut gen = DfsGenerator::new(grid.cell_count(), 0);

        let mut steps = 0;
        while let Some(_step) = gen.step(&mut grid, &mut rng) {
            steps += 1;
        }
        assert!(gen.is_done());
        assert!(steps > 0);

        // A perfect maze on a 10x10 grid has exactly 99 removed walls
        // (N*M - 1 passages for a spanning tree).
        let mut open_passages = 0;
        for cell in 0..grid.cell_count() {
            let (col, row) = grid.cell_coords(cell);
            // Count only East and South to avoid double counting
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
    fn test_dfs_deterministic() {
        let mut grid1 = RectGrid::new(5, 5);
        let mut grid2 = RectGrid::new(5, 5);
        let mut rng1 = Xoshiro256::new(123);
        let mut rng2 = Xoshiro256::new(123);
        let mut gen1 = DfsGenerator::new(25, 0);
        let mut gen2 = DfsGenerator::new(25, 0);

        while let (Some(s1), Some(s2)) = (gen1.step(&mut grid1, &mut rng1), gen2.step(&mut grid2, &mut rng2)) {
            assert_eq!(s1.cell, s2.cell);
        }

        // Verify both grids are identical
        for cell in 0..25 {
            assert_eq!(grid1.wall_bits(cell), grid2.wall_bits(cell));
        }
    }

    #[test]
    fn test_dfs_all_cells_visited() {
        let mut grid = RectGrid::new(8, 8);
        let mut rng = Xoshiro256::new(77);
        let mut gen = DfsGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}

        // In a perfect maze, every cell is reachable from every other cell.
        // Verify with BFS from cell 0.
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
}
