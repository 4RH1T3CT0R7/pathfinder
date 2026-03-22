use std::cmp::Ordering;
use std::collections::BinaryHeap;

use super::{SolveAction, SolveStep, SteppableSolver};
use crate::maze::MazeGrid;

/// A* search maze solver implemented as a state machine.
///
/// Uses f = g + h where g is the actual cost from start and h is the
/// heuristic distance to the goal (delegated to the grid's
/// `heuristic_distance()` method). With an admissible heuristic,
/// A* guarantees the shortest path.
pub struct AStarSolver {
    open_set: BinaryHeap<AStarEntry>,
    g_score: Vec<u32>,
    came_from: Vec<Option<u32>>,
    in_closed: Vec<bool>,
    start: u32,
    end: u32,
    done: bool,
    found: bool,
    solution_path: Vec<u32>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct AStarEntry {
    cell: u32,
    f_score: u32,
}

/// Ordering for `BinaryHeap`: lower f_score = higher priority.
/// Rust's `BinaryHeap` is a max-heap, so we invert the comparison.
impl Ord for AStarEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .f_score
            .cmp(&self.f_score)
            .then_with(|| self.cell.cmp(&other.cell))
    }
}

impl PartialOrd for AStarEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl AStarSolver {
    pub fn new(maze: &dyn MazeGrid, start: u32, end: u32) -> Self {
        let cell_count = maze.cell_count() as usize;

        let mut g_score = vec![u32::MAX; cell_count];
        g_score[start as usize] = 0;

        let h = maze.heuristic_distance(start, end);
        let mut open_set = BinaryHeap::new();
        open_set.push(AStarEntry {
            cell: start,
            f_score: h,
        });

        AStarSolver {
            open_set,
            g_score,
            came_from: vec![None; cell_count],
            in_closed: vec![false; cell_count],
            start,
            end,
            done: false,
            found: false,
            solution_path: Vec::new(),
        }
    }

    fn reconstruct_path(&mut self) {
        self.solution_path.clear();
        let mut current = self.end;
        self.solution_path.push(current);
        while current != self.start {
            match self.came_from[current as usize] {
                Some(p) => {
                    current = p;
                    self.solution_path.push(current);
                }
                None => break,
            }
        }
        self.solution_path.reverse();
    }
}

impl SteppableSolver for AStarSolver {
    fn step(&mut self, maze: &dyn MazeGrid) -> Option<SolveStep> {
        if self.done {
            return None;
        }

        // Skip entries that have already been closed (stale duplicates in the heap)
        let current = loop {
            match self.open_set.pop() {
                Some(entry) => {
                    if !self.in_closed[entry.cell as usize] {
                        break entry.cell;
                    }
                    // stale entry, skip
                }
                None => {
                    self.done = true;
                    return None;
                }
            }
        };

        if current == self.end {
            self.done = true;
            self.found = true;
            self.reconstruct_path();
            return Some(SolveStep {
                cell: current,
                action: SolveAction::FoundGoal,
            });
        }

        self.in_closed[current as usize] = true;
        let current_g = self.g_score[current as usize];

        // Explore neighbors through open passages
        for (neighbor, dir) in maze.neighbors(current) {
            if !maze.has_wall(current, dir) {
                if self.in_closed[neighbor as usize] {
                    continue;
                }
                let tentative_g = current_g + 1;
                if tentative_g < self.g_score[neighbor as usize] {
                    self.g_score[neighbor as usize] = tentative_g;
                    self.came_from[neighbor as usize] = Some(current);
                    let h = maze.heuristic_distance(neighbor, self.end);
                    self.open_set.push(AStarEntry {
                        cell: neighbor,
                        f_score: tentative_g + h,
                    });
                }
            }
        }

        Some(SolveStep {
            cell: current,
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
        self.g_score.fill(u32::MAX);
        self.g_score[start as usize] = 0;
        self.came_from.fill(None);
        self.in_closed.fill(false);
        self.open_set.clear();

        // Note: heuristic will be computed via maze ref in step(), but for the
        // initial entry we don't have a maze ref here. We use 0 as a safe
        // lower bound -- the entry will still be popped first since the heap
        // is empty.
        self.open_set.push(AStarEntry {
            cell: start,
            f_score: 0,
        });

        self.start = start;
        self.end = end;
        self.done = false;
        self.found = false;
        self.solution_path.clear();
    }

    fn is_done(&self) -> bool {
        self.done
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::{DfsGenerator, SteppableGenerator};
    use crate::maze::{Direction, RectGrid};
    use crate::rng::Xoshiro256;
    use crate::solve::BfsSolver;

    fn generate_maze(width: u32, height: u32, seed: u64) -> RectGrid {
        let mut grid = RectGrid::new(width, height);
        let mut rng = Xoshiro256::new(seed);
        let mut gen = DfsGenerator::new(grid.cell_count(), 0);
        while gen.step(&mut grid, &mut rng).is_some() {}
        grid
    }

    #[test]
    fn test_astar_finds_path() {
        let grid = generate_maze(10, 10, 42);
        let start = 0;
        let end = grid.cell_count() - 1;
        let mut solver = AStarSolver::new(&grid, start, end);

        let mut steps = 0;
        while let Some(_step) = solver.step(&grid) {
            steps += 1;
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
    fn test_astar_path_is_valid() {
        let grid = generate_maze(8, 8, 99);
        let start = 0;
        let end = 63;
        let mut solver = AStarSolver::new(&grid, start, end);

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
    fn test_astar_start_equals_end() {
        let grid = generate_maze(5, 5, 1);
        let mut solver = AStarSolver::new(&grid, 0, 0);

        let step = solver.step(&grid);
        assert!(step.is_some());
        assert!(matches!(step.unwrap().action, SolveAction::FoundGoal));
        assert!(solver.is_done());
        assert_eq!(solver.path().unwrap(), &[0]);
    }

    #[test]
    fn test_astar_shortest_path_matches_bfs() {
        // A* with admissible heuristic must produce the same path length as BFS
        for seed in 0..20 {
            let grid = generate_maze(10, 10, seed);
            let start = 0;
            let end = grid.cell_count() - 1;

            let mut bfs = BfsSolver::new(grid.cell_count(), start, end);
            while bfs.step(&grid).is_some() {}

            let mut astar = AStarSolver::new(&grid, start, end);
            while astar.step(&grid).is_some() {}

            let bfs_path = bfs.path().unwrap();
            let astar_path = astar.path().unwrap();

            assert_eq!(
                bfs_path.len(),
                astar_path.len(),
                "A* path length ({}) != BFS path length ({}) for seed {}",
                astar_path.len(),
                bfs_path.len(),
                seed
            );
        }
    }

    #[test]
    fn test_astar_on_corridor() {
        let mut grid = RectGrid::new(3, 1);
        grid.remove_wall(0, Direction::EAST);
        grid.remove_wall(1, Direction::EAST);

        let mut solver = AStarSolver::new(&grid, 0, 2);
        while solver.step(&grid).is_some() {}

        let path = solver.path().unwrap();
        assert_eq!(path, &[0, 1, 2]);
    }

    #[test]
    fn test_astar_reset_works() {
        let grid = generate_maze(6, 6, 42);
        let cell_count = grid.cell_count();
        let mut solver = AStarSolver::new(&grid, 0, cell_count - 1);

        while solver.step(&grid).is_some() {}
        assert!(solver.path().is_some());

        // Reset and solve again with different endpoints
        solver.reset(5, 10);
        assert!(!solver.is_done());
        assert!(solver.path().is_none());

        while solver.step(&grid).is_some() {}
        assert!(solver.is_done());
        assert!(solver.path().is_some());
        let path = solver.path().unwrap();
        assert_eq!(path[0], 5);
        assert_eq!(*path.last().unwrap(), 10);
    }

    #[test]
    fn test_astar_fewer_steps_than_bfs() {
        // On larger mazes with a clear target, A* should typically explore
        // fewer or equal cells compared to BFS thanks to the heuristic.
        let grid = generate_maze(20, 20, 42);
        let start = 0;
        let end = grid.cell_count() - 1;

        let mut bfs = BfsSolver::new(grid.cell_count(), start, end);
        let mut bfs_steps = 0u32;
        while bfs.step(&grid).is_some() {
            bfs_steps += 1;
        }

        let mut astar = AStarSolver::new(&grid, start, end);
        let mut astar_steps = 0u32;
        while astar.step(&grid).is_some() {
            astar_steps += 1;
        }

        // A* should explore no more cells than BFS (or very close)
        // In a perfect maze (tree), A* with Manhattan heuristic should
        // typically explore fewer cells.
        assert!(
            astar_steps <= bfs_steps,
            "A* ({} steps) should not explore more cells than BFS ({} steps)",
            astar_steps,
            bfs_steps
        );
    }
}
