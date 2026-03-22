use super::{GenAction, GenStep, SteppableGenerator};
use crate::maze::MazeGrid;
use crate::rng::Xoshiro256;

/// Randomized Kruskal's algorithm maze generator, implemented as a state machine.
///
/// The algorithm works by treating every interior wall (edge between two adjacent cells)
/// as an element in a shuffled list. On each step, the next edge is examined: if the two
/// cells belong to different disjoint sets, the wall is removed and the sets are merged
/// via union-find. This guarantees a uniform spanning tree (perfect maze).
pub struct KruskalGenerator {
    /// Union-Find parent array (one entry per cell).
    parent: Vec<u32>,
    /// Union-Find rank array for union-by-rank.
    rank: Vec<u8>,
    /// Shuffled list of edges: (cell_a, cell_b, direction from a to b).
    edges: Vec<(u32, u32, crate::maze::Direction)>,
    /// Current index into the edge list.
    cursor: usize,
    /// Whether generation is complete.
    done: bool,
    /// Whether edges have been initialised (deferred until first `step` call
    /// so that we can read the grid dimensions from the `MazeGrid`).
    initialised: bool,
}

impl KruskalGenerator {
    pub fn new(cell_count: u32, _start_cell: u32) -> Self {
        let n = cell_count as usize;
        KruskalGenerator {
            parent: (0..cell_count).collect(),
            rank: vec![0; n],
            edges: Vec::new(),
            cursor: 0,
            done: false,
            initialised: false,
        }
    }

    // ---- Union-Find helpers ------------------------------------------------

    fn find(&mut self, mut x: u32) -> u32 {
        while self.parent[x as usize] != x {
            // Path splitting (one-pass path compression).
            let p = self.parent[x as usize];
            self.parent[x as usize] = self.parent[p as usize];
            x = p;
        }
        x
    }

    fn union(&mut self, a: u32, b: u32) -> bool {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb {
            return false;
        }
        // Union by rank.
        match self.rank[ra as usize].cmp(&self.rank[rb as usize]) {
            std::cmp::Ordering::Less => self.parent[ra as usize] = rb,
            std::cmp::Ordering::Greater => self.parent[rb as usize] = ra,
            std::cmp::Ordering::Equal => {
                self.parent[rb as usize] = ra;
                self.rank[ra as usize] += 1;
            }
        }
        true
    }

    // ---- Edge initialisation -----------------------------------------------

    fn init_edges(&mut self, maze: &dyn MazeGrid, rng: &mut Xoshiro256) {
        // Collect all interior edges exactly once.
        // For each cell, add edges to neighbors with higher index to avoid
        // double-counting. This works for any topology.
        let mut edges = Vec::new();
        for cell in 0..maze.cell_count() {
            for (neighbor, dir) in maze.neighbors(cell) {
                if neighbor > cell {
                    edges.push((cell, neighbor, dir));
                }
            }
        }
        rng.shuffle(&mut edges);
        self.edges = edges;
        self.initialised = true;
    }
}

impl SteppableGenerator for KruskalGenerator {
    fn step(&mut self, maze: &mut dyn MazeGrid, rng: &mut Xoshiro256) -> Option<GenStep> {
        if self.done {
            return None;
        }

        if !self.initialised {
            self.init_edges(maze, rng);
        }

        // Iterate through edges until we find one connecting two different sets
        // or we exhaust the list.
        while self.cursor < self.edges.len() {
            let (cell_a, cell_b, dir) = self.edges[self.cursor];
            self.cursor += 1;

            if self.union(cell_a, cell_b) {
                maze.remove_wall(cell_a, dir);
                return Some(GenStep {
                    cell: cell_a,
                    action: GenAction::RemoveWall(dir),
                });
            }
        }

        // All edges processed.
        self.done = true;
        None
    }

    fn reset(&mut self, _start_cell: u32) {
        let n = self.parent.len();
        for i in 0..n {
            self.parent[i] = i as u32;
        }
        self.rank.fill(0);
        self.edges.clear();
        self.cursor = 0;
        self.done = false;
        self.initialised = false;
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
    /// Only counts East and South to avoid double-counting.
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
    fn test_kruskal_generates_perfect_maze() {
        let mut grid = RectGrid::new(10, 10);
        let mut rng = Xoshiro256::new(42);
        let mut gen = KruskalGenerator::new(grid.cell_count(), 0);

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
    fn test_kruskal_deterministic() {
        let mut grid1 = RectGrid::new(5, 5);
        let mut grid2 = RectGrid::new(5, 5);
        let mut rng1 = Xoshiro256::new(123);
        let mut rng2 = Xoshiro256::new(123);
        let mut gen1 = KruskalGenerator::new(25, 0);
        let mut gen2 = KruskalGenerator::new(25, 0);

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
    fn test_kruskal_all_cells_reachable() {
        let mut grid = RectGrid::new(8, 8);
        let mut rng = Xoshiro256::new(77);
        let mut gen = KruskalGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}

        assert!(all_cells_reachable(&grid), "Not all cells are reachable");
    }

    #[test]
    fn test_kruskal_small_grid() {
        let mut grid = RectGrid::new(2, 2);
        let mut rng = Xoshiro256::new(1);
        let mut gen = KruskalGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}

        assert!(gen.is_done());
        assert_eq!(count_passages(&grid), 3); // 2*2 - 1
        assert!(all_cells_reachable(&grid));
    }

    #[test]
    fn test_kruskal_rectangular_grid() {
        let mut grid = RectGrid::new(7, 3);
        let mut rng = Xoshiro256::new(999);
        let mut gen = KruskalGenerator::new(grid.cell_count(), 0);

        while gen.step(&mut grid, &mut rng).is_some() {}

        assert!(gen.is_done());
        assert_eq!(count_passages(&grid), 20); // 7*3 - 1
        assert!(all_cells_reachable(&grid));
    }

    #[test]
    fn test_kruskal_reset() {
        let mut grid = RectGrid::new(5, 5);
        let mut rng = Xoshiro256::new(42);
        let mut gen = KruskalGenerator::new(grid.cell_count(), 0);

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
