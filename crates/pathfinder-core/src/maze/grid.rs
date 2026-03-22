use super::{Direction, MazeGrid, Topology};

/// The four rectangular directions, stored as a static array for `directions()`.
static RECT_DIRECTIONS: [Direction; 4] = [
    Direction::NORTH,
    Direction::EAST,
    Direction::SOUTH,
    Direction::WEST,
];

/// Rectangular grid maze. Each cell stores a 4-bit wall bitmask (N, E, S, W).
pub struct RectGrid {
    width: u32,
    height: u32,
    /// Wall bitmask per cell. Bits: 0=North, 1=East, 2=South, 3=West.
    walls: Vec<u8>,
}

impl RectGrid {
    pub fn new(width: u32, height: u32) -> Self {
        let count = (width * height) as usize;
        let mut grid = RectGrid {
            width,
            height,
            walls: vec![0; count],
        };
        grid.fill_walls();
        grid
    }

    fn neighbor_in_direction(&self, cell: u32, dir: Direction) -> Option<u32> {
        let (col, row) = self.cell_coords(cell);
        if dir == Direction::NORTH && row > 0 {
            Some(self.cell_index(col, row - 1))
        } else if dir == Direction::SOUTH && row < self.height - 1 {
            Some(self.cell_index(col, row + 1))
        } else if dir == Direction::WEST && col > 0 {
            Some(self.cell_index(col - 1, row))
        } else if dir == Direction::EAST && col < self.width - 1 {
            Some(self.cell_index(col + 1, row))
        } else {
            None
        }
    }
}

impl MazeGrid for RectGrid {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn cell_count(&self) -> u32 {
        self.width * self.height
    }

    fn topology(&self) -> Topology {
        Topology::Rectangular
    }

    fn directions(&self) -> &'static [Direction] {
        &RECT_DIRECTIONS
    }

    fn opposite(&self, dir: Direction) -> Direction {
        match dir.0 {
            0 => Direction::SOUTH,  // NORTH -> SOUTH
            1 => Direction::WEST,   // EAST -> WEST
            2 => Direction::NORTH,  // SOUTH -> NORTH
            3 => Direction::EAST,   // WEST -> EAST
            _ => dir,
        }
    }

    fn wall_bit(&self, dir: Direction) -> u8 {
        1 << dir.0
    }

    fn heuristic_distance(&self, from: u32, to: u32) -> u32 {
        let (from_col, from_row) = self.cell_coords(from);
        let (to_col, to_row) = self.cell_coords(to);
        from_col.abs_diff(to_col) + from_row.abs_diff(to_row)
    }

    fn cell_position(&self, cell: u32) -> (f64, f64) {
        let (col, row) = self.cell_coords(cell);
        (col as f64, row as f64)
    }

    fn wall_bits(&self, cell: u32) -> u8 {
        self.walls[cell as usize]
    }

    fn wall_bits_ptr(&self) -> *const u8 {
        self.walls.as_ptr()
    }

    fn neighbors(&self, cell: u32) -> Vec<(u32, Direction)> {
        let mut result = Vec::with_capacity(4);
        for &dir in Direction::all_rect() {
            if let Some(neighbor) = self.neighbor_in_direction(cell, dir) {
                result.push((neighbor, dir));
            }
        }
        result
    }

    fn has_wall(&self, cell: u32, dir: Direction) -> bool {
        self.walls[cell as usize] & self.wall_bit(dir) != 0
    }

    fn remove_wall(&mut self, cell: u32, dir: Direction) {
        let bit = self.wall_bit(dir);
        self.walls[cell as usize] &= !bit;
        if let Some(neighbor) = self.neighbor_in_direction(cell, dir) {
            let opp_bit = self.wall_bit(self.opposite(dir));
            self.walls[neighbor as usize] &= !opp_bit;
        }
    }

    fn add_wall(&mut self, cell: u32, dir: Direction) {
        let bit = self.wall_bit(dir);
        self.walls[cell as usize] |= bit;
        if let Some(neighbor) = self.neighbor_in_direction(cell, dir) {
            let opp_bit = self.wall_bit(self.opposite(dir));
            self.walls[neighbor as usize] |= opp_bit;
        }
    }

    fn fill_walls(&mut self) {
        for cell in 0..self.cell_count() {
            let (col, row) = self.cell_coords(cell);
            let mut bits = 0u8;
            bits |= self.wall_bit(Direction::NORTH);
            bits |= self.wall_bit(Direction::SOUTH);
            bits |= self.wall_bit(Direction::EAST);
            bits |= self.wall_bit(Direction::WEST);
            let _ = (col, row); // suppress unused warning
            self.walls[cell as usize] = bits;
        }
    }

    fn clear_walls(&mut self) {
        self.walls.fill(0);
    }

    fn cell_index(&self, col: u32, row: u32) -> u32 {
        row * self.width + col
    }

    fn cell_coords(&self, index: u32) -> (u32, u32) {
        (index % self.width, index / self.width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_grid_all_walls() {
        let grid = RectGrid::new(5, 5);
        assert_eq!(grid.cell_count(), 25);
        for cell in 0..25 {
            for &dir in Direction::all_rect() {
                assert!(grid.has_wall(cell, dir));
            }
        }
    }

    #[test]
    fn test_coords_roundtrip() {
        let grid = RectGrid::new(10, 8);
        for row in 0..8 {
            for col in 0..10 {
                let idx = grid.cell_index(col, row);
                let (c, r) = grid.cell_coords(idx);
                assert_eq!((c, r), (col, row));
            }
        }
    }

    #[test]
    fn test_remove_wall_symmetric() {
        let mut grid = RectGrid::new(5, 5);
        let cell = grid.cell_index(2, 2);
        let neighbor = grid.cell_index(3, 2); // east neighbor

        assert!(grid.has_wall(cell, Direction::EAST));
        assert!(grid.has_wall(neighbor, Direction::WEST));

        grid.remove_wall(cell, Direction::EAST);

        assert!(!grid.has_wall(cell, Direction::EAST));
        assert!(!grid.has_wall(neighbor, Direction::WEST));
    }

    #[test]
    fn test_neighbors_corner() {
        let grid = RectGrid::new(5, 5);
        // Top-left corner (0,0) should have only East and South neighbors
        let neighbors = grid.neighbors(0);
        assert_eq!(neighbors.len(), 2);

        let dirs: Vec<Direction> = neighbors.iter().map(|&(_, d)| d).collect();
        assert!(dirs.contains(&Direction::EAST));
        assert!(dirs.contains(&Direction::SOUTH));
    }

    #[test]
    fn test_neighbors_center() {
        let grid = RectGrid::new(5, 5);
        let cell = grid.cell_index(2, 2);
        let neighbors = grid.neighbors(cell);
        assert_eq!(neighbors.len(), 4);
    }

    #[test]
    fn test_clear_walls() {
        let mut grid = RectGrid::new(3, 3);
        grid.clear_walls();
        for cell in 0..9 {
            for &dir in Direction::all_rect() {
                assert!(!grid.has_wall(cell, dir));
            }
        }
    }

    #[test]
    fn test_add_wall_symmetric() {
        let mut grid = RectGrid::new(3, 3);
        grid.clear_walls();
        let cell = grid.cell_index(1, 1);
        grid.add_wall(cell, Direction::NORTH);

        assert!(grid.has_wall(cell, Direction::NORTH));
        let north_neighbor = grid.cell_index(1, 0);
        assert!(grid.has_wall(north_neighbor, Direction::SOUTH));
    }
}
