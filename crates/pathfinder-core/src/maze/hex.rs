use super::{Direction, MazeGrid, Topology};

/// The six hexagonal directions (pointy-top, odd-row offset).
pub const HEX_E: Direction = Direction(0);
pub const HEX_NE: Direction = Direction(1);
pub const HEX_NW: Direction = Direction(2);
pub const HEX_W: Direction = Direction(3);
pub const HEX_SW: Direction = Direction(4);
pub const HEX_SE: Direction = Direction(5);

static HEX_DIRECTIONS: [Direction; 6] = [HEX_E, HEX_NE, HEX_NW, HEX_W, HEX_SW, HEX_SE];

/// All six walls present: bits 0..5 set.
const ALL_HEX_WALLS: u8 = 0b0011_1111;

/// Pointy-top hexagonal grid with odd-row offset coordinates.
///
/// Each cell stores a 6-bit wall bitmask — one bit per hex direction.
/// Bit layout: `E(0) NE(1) NW(2) W(3) SW(4) SE(5)`.
///
/// Odd-row offset means that odd rows are shifted half a cell to the right,
/// which affects the column offsets when computing neighbors for diagonal
/// directions (NE, NW, SW, SE).
pub struct HexGrid {
    width: u32,
    height: u32,
    /// Wall bitmask per cell. Bits 0..5 correspond to the six hex directions.
    walls: Vec<u8>,
}

/// Neighbor offsets for even rows (`row % 2 == 0`).
///
/// Order: E, NE, NW, W, SW, SE — matching the `Direction` constants.
const EVEN_ROW_OFFSETS: [(i32, i32); 6] = [
    (1, 0),   // E
    (0, -1),  // NE
    (-1, -1), // NW
    (-1, 0),  // W
    (-1, 1),  // SW
    (0, 1),   // SE
];

/// Neighbor offsets for odd rows (`row % 2 == 1`).
const ODD_ROW_OFFSETS: [(i32, i32); 6] = [
    (1, 0),  // E
    (1, -1), // NE
    (0, -1), // NW
    (-1, 0), // W
    (0, 1),  // SW
    (1, 1),  // SE
];

impl HexGrid {
    pub fn new(width: u32, height: u32) -> Self {
        let count = (width * height) as usize;
        let mut grid = HexGrid {
            width,
            height,
            walls: vec![0; count],
        };
        grid.fill_walls();
        grid
    }

    /// Returns the neighbor cell index for a given cell and direction, if it exists.
    fn neighbor_in_direction(&self, cell: u32, dir: Direction) -> Option<u32> {
        let (col, row) = self.cell_coords(cell);
        let offsets = if row % 2 == 0 {
            &EVEN_ROW_OFFSETS
        } else {
            &ODD_ROW_OFFSETS
        };
        let idx = dir.0 as usize;
        if idx >= 6 {
            return None;
        }
        let (dc, dr) = offsets[idx];
        let nc = i64::from(col) + i64::from(dc);
        let nr = i64::from(row) + i64::from(dr);
        if nc >= 0 && nc < i64::from(self.width) && nr >= 0 && nr < i64::from(self.height) {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            Some(self.cell_index(nc as u32, nr as u32))
        } else {
            None
        }
    }

    /// Convert offset coordinates to cube coordinates (q, r, s) where q + r + s = 0.
    fn to_cube(col: u32, row: u32) -> (i64, i64, i64) {
        let col = i64::from(col);
        let row = i64::from(row);
        let q = col - (row - (row & 1)) / 2;
        let r = row;
        let s = -q - r;
        (q, r, s)
    }
}

impl MazeGrid for HexGrid {
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
        Topology::Hexagonal
    }

    fn directions(&self) -> &'static [Direction] {
        &HEX_DIRECTIONS
    }

    fn opposite(&self, dir: Direction) -> Direction {
        match dir.0 {
            0 => HEX_W,  // E -> W
            1 => HEX_SW, // NE -> SW
            2 => HEX_SE, // NW -> SE
            3 => HEX_E,  // W -> E
            4 => HEX_NE, // SW -> NE
            5 => HEX_NW, // SE -> NW
            _ => dir,
        }
    }

    fn wall_bit(&self, dir: Direction) -> u8 {
        1 << dir.0
    }

    #[allow(clippy::cast_possible_truncation)]
    fn heuristic_distance(&self, from: u32, to: u32) -> u32 {
        let (fc, fr) = self.cell_coords(from);
        let (tc, tr) = self.cell_coords(to);
        let (q1, r1, s1) = HexGrid::to_cube(fc, fr);
        let (q2, r2, s2) = HexGrid::to_cube(tc, tr);
        let dq = (q1 - q2).unsigned_abs() as u32;
        let dr = (r1 - r2).unsigned_abs() as u32;
        let ds = (s1 - s2).unsigned_abs() as u32;
        dq.max(dr).max(ds)
    }

    fn cell_position(&self, cell: u32) -> (f64, f64) {
        let (col, row) = self.cell_coords(cell);
        let size = 1.0_f64;
        let x = size * 3.0_f64.sqrt() * (f64::from(col) + 0.5 * f64::from(row & 1));
        let y = size * 1.5 * f64::from(row);
        (x, y)
    }

    fn wall_bits(&self, cell: u32) -> u8 {
        self.walls[cell as usize]
    }

    fn wall_bits_ptr(&self) -> *const u8 {
        self.walls.as_ptr()
    }

    fn neighbors(&self, cell: u32) -> Vec<(u32, Direction)> {
        let mut result = Vec::with_capacity(6);
        for &dir in &HEX_DIRECTIONS {
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
        for w in &mut self.walls {
            *w = ALL_HEX_WALLS;
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
        let grid = HexGrid::new(5, 5);
        assert_eq!(grid.cell_count(), 25);
        for cell in 0..25 {
            for &dir in &HEX_DIRECTIONS {
                assert!(grid.has_wall(cell, dir), "cell {cell} missing wall {dir:?}");
            }
        }
    }

    #[test]
    fn test_coords_roundtrip() {
        let grid = HexGrid::new(8, 6);
        for row in 0..6 {
            for col in 0..8 {
                let idx = grid.cell_index(col, row);
                let (c, r) = grid.cell_coords(idx);
                assert_eq!((c, r), (col, row));
            }
        }
    }

    #[test]
    fn test_remove_wall_symmetric() {
        let mut grid = HexGrid::new(5, 5);
        // Cell (2,2) even row, remove E wall -> neighbor is (3,2)
        let cell = grid.cell_index(2, 2);
        let neighbor = grid.cell_index(3, 2);

        assert!(grid.has_wall(cell, HEX_E));
        assert!(grid.has_wall(neighbor, HEX_W));

        grid.remove_wall(cell, HEX_E);

        assert!(!grid.has_wall(cell, HEX_E));
        assert!(!grid.has_wall(neighbor, HEX_W));
    }

    #[test]
    fn test_corner_neighbors_count() {
        let grid = HexGrid::new(5, 5);
        // Top-left corner (0,0) even row: possible neighbors
        // E(1,0): valid, NE(0,-1): invalid, NW(-1,-1): invalid,
        // W(-1,0): invalid, SW(-1,1): invalid, SE(0,1): valid
        let neighbors = grid.neighbors(0);
        assert_eq!(neighbors.len(), 2);
    }

    #[test]
    fn test_center_neighbors_count() {
        let grid = HexGrid::new(5, 5);
        let cell = grid.cell_index(2, 2);
        let neighbors = grid.neighbors(cell);
        assert_eq!(neighbors.len(), 6);
    }

    #[test]
    fn test_clear_walls() {
        let mut grid = HexGrid::new(3, 3);
        grid.clear_walls();
        for cell in 0..9 {
            for &dir in &HEX_DIRECTIONS {
                assert!(!grid.has_wall(cell, dir));
            }
        }
    }

    #[test]
    fn test_add_wall_symmetric() {
        let mut grid = HexGrid::new(3, 3);
        grid.clear_walls();
        // Cell (1,1) odd row, add NE wall -> neighbor at (2,0)
        let cell = grid.cell_index(1, 1);
        grid.add_wall(cell, HEX_NE);

        assert!(grid.has_wall(cell, HEX_NE));
        let ne_neighbor = grid.cell_index(2, 0);
        assert!(grid.has_wall(ne_neighbor, HEX_SW));
    }

    #[test]
    fn test_heuristic_distance() {
        let grid = HexGrid::new(5, 5);
        // Same cell => distance 0
        let cell = grid.cell_index(2, 2);
        assert_eq!(grid.heuristic_distance(cell, cell), 0);

        // Adjacent cell (E neighbor) => distance 1
        let east = grid.cell_index(3, 2);
        assert_eq!(grid.heuristic_distance(cell, east), 1);

        // Two steps away: (0,0) to (2,0) going E twice => distance 2
        let a = grid.cell_index(0, 0);
        let b = grid.cell_index(2, 0);
        assert_eq!(grid.heuristic_distance(a, b), 2);
    }

    #[test]
    fn test_opposite_directions() {
        let grid = HexGrid::new(1, 1);
        assert_eq!(grid.opposite(HEX_E), HEX_W);
        assert_eq!(grid.opposite(HEX_W), HEX_E);
        assert_eq!(grid.opposite(HEX_NE), HEX_SW);
        assert_eq!(grid.opposite(HEX_SW), HEX_NE);
        assert_eq!(grid.opposite(HEX_NW), HEX_SE);
        assert_eq!(grid.opposite(HEX_SE), HEX_NW);
    }

    #[test]
    fn test_odd_row_neighbors() {
        let grid = HexGrid::new(5, 5);
        // Cell (2,1) odd row — should have 6 neighbors
        let cell = grid.cell_index(2, 1);
        let neighbors = grid.neighbors(cell);
        assert_eq!(neighbors.len(), 6);

        // Verify specific neighbors for odd row offsets
        let neighbor_cells: Vec<u32> = neighbors.iter().map(|&(n, _)| n).collect();
        assert!(neighbor_cells.contains(&grid.cell_index(3, 1))); // E
        assert!(neighbor_cells.contains(&grid.cell_index(3, 0))); // NE
        assert!(neighbor_cells.contains(&grid.cell_index(2, 0))); // NW
        assert!(neighbor_cells.contains(&grid.cell_index(1, 1))); // W
        assert!(neighbor_cells.contains(&grid.cell_index(2, 2))); // SW
        assert!(neighbor_cells.contains(&grid.cell_index(3, 2))); // SE
    }

    #[test]
    fn test_remove_wall_diagonal_symmetric() {
        let mut grid = HexGrid::new(5, 5);
        // Even row cell (2,2): NE neighbor is (2,1)
        let cell = grid.cell_index(2, 2);
        let ne_neighbor = grid.cell_index(2, 1);

        grid.remove_wall(cell, HEX_NE);

        assert!(!grid.has_wall(cell, HEX_NE));
        assert!(!grid.has_wall(ne_neighbor, HEX_SW));
    }

    #[test]
    fn test_cell_position_origin() {
        let grid = HexGrid::new(5, 5);
        let (x, y) = grid.cell_position(0);
        assert!((x - 0.0).abs() < 1e-9);
        assert!((y - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_top_right_corner_neighbors() {
        let grid = HexGrid::new(5, 5);
        // Top-right corner (4,0) even row:
        // E(5,0): invalid, NE(4,-1): invalid, NW(3,-1): invalid,
        // W(3,0): valid, SW(3,1): valid, SE(4,1): valid
        let cell = grid.cell_index(4, 0);
        let neighbors = grid.neighbors(cell);
        assert_eq!(neighbors.len(), 3);
    }
}
