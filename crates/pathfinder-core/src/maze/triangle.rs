use super::{Direction, MazeGrid, Topology};

/// Triangle grid directions.
pub const TRI_LEFT: Direction = Direction(0);
pub const TRI_RIGHT: Direction = Direction(1);
pub const TRI_BASE: Direction = Direction(2);

static TRI_DIRECTIONS: [Direction; 3] = [TRI_LEFT, TRI_RIGHT, TRI_BASE];

/// All three walls present: bits 0..2 set.
const ALL_TRI_WALLS: u8 = 0b0000_0111;

/// Triangular grid with alternating up-pointing and down-pointing triangles.
///
/// A cell at `(col, row)` is **up-pointing** when `(col + row) % 2 == 0` and
/// **down-pointing** when `(col + row) % 2 == 1`.
///
/// Each cell has exactly 3 edges and therefore at most 3 neighbors:
/// - **LEFT**: the cell at `(col - 1, row)` — shares a slanted edge.
/// - **RIGHT**: the cell at `(col + 1, row)` — shares a slanted edge.
/// - **BASE**: the horizontal edge.
///   - For an up-pointing triangle the base is at the bottom, so the base
///     neighbor is `(col, row + 1)`.
///   - For a down-pointing triangle the base is at the top, so the base
///     neighbor is `(col, row - 1)`.
///
/// Wall bitmask: `LEFT(0) RIGHT(1) BASE(2)`.
pub struct TriangleGrid {
    width: u32,
    height: u32,
    /// Wall bitmask per cell. Bits 0..2 correspond to LEFT, RIGHT, BASE.
    walls: Vec<u8>,
}

impl TriangleGrid {
    pub fn new(width: u32, height: u32) -> Self {
        let count = (width * height) as usize;
        let mut grid = TriangleGrid {
            width,
            height,
            walls: vec![0; count],
        };
        grid.fill_walls();
        grid
    }

    /// Whether the cell at `(col, row)` is up-pointing.
    fn is_up(col: u32, row: u32) -> bool {
        (col + row).is_multiple_of(2)
    }

    /// Returns the neighbor cell index for a given cell and direction, if it exists.
    fn neighbor_in_direction(&self, cell: u32, dir: Direction) -> Option<u32> {
        let (col, row) = self.cell_coords(cell);
        match dir.0 {
            // LEFT
            0 => {
                if col > 0 {
                    Some(self.cell_index(col - 1, row))
                } else {
                    None
                }
            }
            // RIGHT
            1 => {
                if col < self.width - 1 {
                    Some(self.cell_index(col + 1, row))
                } else {
                    None
                }
            }
            // BASE
            2 => {
                if Self::is_up(col, row) {
                    // Up-pointing: base is bottom edge -> neighbor at (col, row+1)
                    if row < self.height - 1 {
                        Some(self.cell_index(col, row + 1))
                    } else {
                        None
                    }
                } else {
                    // Down-pointing: base is top edge -> neighbor at (col, row-1)
                    if row > 0 {
                        Some(self.cell_index(col, row - 1))
                    } else {
                        None
                    }
                }
            }
            _ => None,
        }
    }
}

impl MazeGrid for TriangleGrid {
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
        Topology::Triangular
    }

    fn directions(&self) -> &'static [Direction] {
        &TRI_DIRECTIONS
    }

    fn opposite(&self, dir: Direction) -> Direction {
        match dir.0 {
            0 => TRI_RIGHT, // LEFT -> RIGHT
            1 => TRI_LEFT,  // RIGHT -> LEFT
            2 => TRI_BASE,  // BASE -> BASE (base edges face each other)
            _ => dir,
        }
    }

    fn wall_bit(&self, dir: Direction) -> u8 {
        1 << dir.0
    }

    fn heuristic_distance(&self, from: u32, to: u32) -> u32 {
        let (x1, y1) = self.cell_position(from);
        let (x2, y2) = self.cell_position(to);
        let dx = x1 - x2;
        let dy = y1 - y2;
        // Euclidean distance, truncated to integer for the heuristic.
        // This is admissible because the shortest graph path is always >= Euclidean.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        { (dx * dx + dy * dy).sqrt() as u32 }
    }

    fn cell_position(&self, cell: u32) -> (f64, f64) {
        let (col, row) = self.cell_coords(cell);
        // Each triangle occupies half a unit width. The center x is at:
        let x = f64::from(col) * 0.5 + 0.25;
        // Vertical: row height is sqrt(3)/2 (equilateral triangle height).
        // Center y offset depends on orientation.
        let row_height = 3.0_f64.sqrt() / 2.0;
        let y = if Self::is_up(col, row) {
            // Up-pointing: centroid is at 1/3 from base (bottom), i.e. 2/3 from top
            f64::from(row) * row_height + row_height * (2.0 / 3.0)
        } else {
            // Down-pointing: centroid is at 1/3 from base (top), i.e. 1/3 from top
            f64::from(row) * row_height + row_height * (1.0 / 3.0)
        };
        (x, y)
    }

    fn wall_bits(&self, cell: u32) -> u8 {
        self.walls[cell as usize]
    }

    fn wall_bits_ptr(&self) -> *const u8 {
        self.walls.as_ptr()
    }

    fn neighbors(&self, cell: u32) -> Vec<(u32, Direction)> {
        let mut result = Vec::with_capacity(3);
        for &dir in &TRI_DIRECTIONS {
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
            *w = ALL_TRI_WALLS;
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
        let grid = TriangleGrid::new(6, 4);
        assert_eq!(grid.cell_count(), 24);
        for cell in 0..24 {
            for &dir in &TRI_DIRECTIONS {
                assert!(grid.has_wall(cell, dir), "cell {cell} missing wall {dir:?}");
            }
        }
    }

    #[test]
    fn test_coords_roundtrip() {
        let grid = TriangleGrid::new(7, 5);
        for row in 0..5 {
            for col in 0..7 {
                let idx = grid.cell_index(col, row);
                let (c, r) = grid.cell_coords(idx);
                assert_eq!((c, r), (col, row));
            }
        }
    }

    #[test]
    fn test_remove_wall_symmetric_left_right() {
        let mut grid = TriangleGrid::new(6, 4);
        // Cell (2,0) up-pointing, remove RIGHT -> neighbor is (3,0) down-pointing
        let cell = grid.cell_index(2, 0);
        let neighbor = grid.cell_index(3, 0);

        assert!(grid.has_wall(cell, TRI_RIGHT));
        assert!(grid.has_wall(neighbor, TRI_LEFT));

        grid.remove_wall(cell, TRI_RIGHT);

        assert!(!grid.has_wall(cell, TRI_RIGHT));
        assert!(!grid.has_wall(neighbor, TRI_LEFT));
    }

    #[test]
    fn test_remove_wall_symmetric_base() {
        let mut grid = TriangleGrid::new(6, 4);
        // Cell (0,0) up-pointing, remove BASE -> neighbor is (0,1) down-pointing (base at top)
        let cell = grid.cell_index(0, 0);
        let neighbor = grid.cell_index(0, 1);

        // (0+0)%2 == 0 -> up-pointing, base goes to (0,1)
        // (0+1)%2 == 1 -> down-pointing, base goes to (0,0) — same cell above!
        assert!(grid.has_wall(cell, TRI_BASE));
        assert!(grid.has_wall(neighbor, TRI_BASE));

        grid.remove_wall(cell, TRI_BASE);

        assert!(!grid.has_wall(cell, TRI_BASE));
        assert!(!grid.has_wall(neighbor, TRI_BASE));
    }

    #[test]
    fn test_corner_neighbors_count() {
        let grid = TriangleGrid::new(6, 4);
        // Cell (0,0) up-pointing: LEFT(-1,0) invalid, RIGHT(1,0) valid, BASE(0,1) valid
        let neighbors = grid.neighbors(0);
        assert_eq!(neighbors.len(), 2);
    }

    #[test]
    fn test_center_neighbors_count() {
        let grid = TriangleGrid::new(6, 4);
        // Cell (2,1) — (2+1)%2=1 => down-pointing
        // LEFT(1,1) valid, RIGHT(3,1) valid, BASE(2,0) valid
        let cell = grid.cell_index(2, 1);
        let neighbors = grid.neighbors(cell);
        assert_eq!(neighbors.len(), 3);
    }

    #[test]
    fn test_clear_walls() {
        let mut grid = TriangleGrid::new(4, 3);
        grid.clear_walls();
        for cell in 0..12 {
            for &dir in &TRI_DIRECTIONS {
                assert!(!grid.has_wall(cell, dir));
            }
        }
    }

    #[test]
    fn test_add_wall_symmetric() {
        let mut grid = TriangleGrid::new(6, 4);
        grid.clear_walls();
        // Cell (3,1) — (3+1)%2=0 => up-pointing, add LEFT wall -> neighbor (2,1)
        let cell = grid.cell_index(3, 1);
        grid.add_wall(cell, TRI_LEFT);

        assert!(grid.has_wall(cell, TRI_LEFT));
        let left_neighbor = grid.cell_index(2, 1);
        assert!(grid.has_wall(left_neighbor, TRI_RIGHT));
    }

    #[test]
    fn test_heuristic_distance() {
        let grid = TriangleGrid::new(6, 4);
        // Same cell => distance 0
        let cell = grid.cell_index(2, 2);
        assert_eq!(grid.heuristic_distance(cell, cell), 0);

        // Adjacent cells should have small distance
        let neighbor = grid.cell_index(3, 2);
        let dist = grid.heuristic_distance(cell, neighbor);
        // Adjacent triangle centers are 0.5 apart horizontally, so sqrt distance ~ 0
        // Euclidean dist as u32 will be 0 (< 1.0)
        assert!(dist <= 1);
    }

    #[test]
    fn test_opposite_directions() {
        let grid = TriangleGrid::new(1, 1);
        assert_eq!(grid.opposite(TRI_LEFT), TRI_RIGHT);
        assert_eq!(grid.opposite(TRI_RIGHT), TRI_LEFT);
        assert_eq!(grid.opposite(TRI_BASE), TRI_BASE);
    }

    #[test]
    fn test_up_pointing_orientation() {
        // (col + row) % 2 == 0 => up-pointing
        assert!(TriangleGrid::is_up(0, 0));
        assert!(TriangleGrid::is_up(2, 0));
        assert!(TriangleGrid::is_up(1, 1));
        assert!(!TriangleGrid::is_up(1, 0));
        assert!(!TriangleGrid::is_up(0, 1));
    }

    #[test]
    fn test_down_pointing_base_neighbor_goes_up() {
        let grid = TriangleGrid::new(6, 4);
        // Cell (1,2): (1+2)%2=1 => down-pointing, BASE goes to (1,1)
        let cell = grid.cell_index(1, 2);
        let base_neighbor = grid.neighbor_in_direction(cell, TRI_BASE);
        assert_eq!(base_neighbor, Some(grid.cell_index(1, 1)));
    }

    #[test]
    fn test_up_pointing_base_neighbor_goes_down() {
        let grid = TriangleGrid::new(6, 4);
        // Cell (2,0): (2+0)%2=0 => up-pointing, BASE goes to (2,1)
        let cell = grid.cell_index(2, 0);
        let base_neighbor = grid.neighbor_in_direction(cell, TRI_BASE);
        assert_eq!(base_neighbor, Some(grid.cell_index(2, 1)));
    }

    #[test]
    fn test_edge_row_base_neighbors() {
        let grid = TriangleGrid::new(6, 4);
        // Bottom-right up-pointing cell: (4,3) — (4+3)%2=1 down-pointing actually
        // Let's pick (5,3) — (5+3)%2=0 => up-pointing, BASE=(5,4) but row 4 doesn't exist
        let cell = grid.cell_index(5, 3);
        let base = grid.neighbor_in_direction(cell, TRI_BASE);
        assert_eq!(base, None);

        // Top-row down-pointing cell: (1,0) — (1+0)%2=1 => down-pointing, BASE=(1,-1) invalid
        let cell_top = grid.cell_index(1, 0);
        let base_top = grid.neighbor_in_direction(cell_top, TRI_BASE);
        assert_eq!(base_top, None);
    }
}
