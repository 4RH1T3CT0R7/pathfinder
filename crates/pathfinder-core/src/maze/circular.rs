use super::{Direction, MazeGrid, Topology};

/// Circular maze direction: toward the center ring.
pub const CIRC_INWARD: Direction = Direction(0);
/// Circular maze direction: clockwise within the same ring.
pub const CIRC_CW: Direction = Direction(1);
/// Circular maze direction: away from the center (first outward neighbor).
pub const CIRC_OUTWARD: Direction = Direction(2);
/// Circular maze direction: counter-clockwise within the same ring.
pub const CIRC_CCW: Direction = Direction(3);
/// Circular maze direction: second outward neighbor (when the outer ring has
/// more cells than the current ring, a cell may map to two outward neighbors).
pub const CIRC_OUTWARD2: Direction = Direction(4);

/// The four canonical circular directions used by `directions()`.
static CIRC_DIRECTIONS: [Direction; 4] = [CIRC_INWARD, CIRC_CW, CIRC_OUTWARD, CIRC_CCW];

/// Concentric-ring circular maze grid.
///
/// Ring 0 is the single center cell. Each subsequent ring `r` (for `r >= 1`)
/// contains `6 * r` cells, so the cell count grows proportionally with the
/// circumference, keeping individual cells roughly equal in area.
///
/// # Wall encoding
///
/// * **Center cell (ring 0):** uses 6 wall bits (one per outward edge toward
///   each cell in ring 1). Direction values `Direction(0)` through
///   `Direction(5)` each identify one of the six ring-1 neighbors.
///
/// * **Non-center cells:** use the four canonical directions (`CIRC_INWARD`,
///   `CIRC_CW`, `CIRC_OUTWARD`, `CIRC_CCW`). When a cell has a second outward
///   neighbor, `CIRC_OUTWARD2` (`Direction(4)`) is also used.
pub struct CircularGrid {
    rings: u32,
    cells_per_ring: Vec<u32>,
    /// Prefix sum: `ring_offset[r]` = total number of cells in rings `0..r`.
    ring_offset: Vec<u32>,
    total_cells: u32,
    /// One byte per cell. Bits map to direction constants via `wall_bit()`.
    walls: Vec<u8>,
}

impl CircularGrid {
    /// Creates a new circular maze with the given number of concentric rings.
    ///
    /// `rings` must be at least 1 (the center cell alone). A typical maze uses
    /// 5-15 rings.
    pub fn new(rings: u32) -> Self {
        assert!(rings >= 1, "CircularGrid requires at least 1 ring");

        let mut cells_per_ring = Vec::with_capacity(rings as usize);
        cells_per_ring.push(1); // ring 0
        for r in 1..rings {
            cells_per_ring.push(6 * r);
        }

        let mut ring_offset = Vec::with_capacity(rings as usize + 1);
        ring_offset.push(0);
        let mut acc = 0u32;
        for &count in &cells_per_ring {
            acc += count;
            ring_offset.push(acc);
        }

        let total_cells = acc;
        let mut grid = CircularGrid {
            rings,
            cells_per_ring,
            ring_offset,
            total_cells,
            walls: vec![0; total_cells as usize],
        };
        grid.fill_walls();
        grid
    }

    /// Returns the (ring, position) for a given cell index.
    fn ring_and_pos(&self, cell: u32) -> (u32, u32) {
        // Binary search: find the ring `r` such that
        // ring_offset[r] <= cell < ring_offset[r+1].
        let mut lo = 0u32;
        let mut hi = self.rings;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.ring_offset[mid as usize + 1] <= cell {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let ring = lo;
        let pos = cell - self.ring_offset[ring as usize];
        (ring, pos)
    }

    /// Cell index for a given ring and position within that ring.
    fn cell_at(&self, ring: u32, pos: u32) -> u32 {
        self.ring_offset[ring as usize] + pos
    }

    /// Returns the neighbor cell in a given direction, or `None` if no such
    /// neighbor exists.
    ///
    /// For the center cell, `dir.0` in `0..cells_per_ring[1]` selects the
    /// specific ring-1 neighbor. For other cells the standard INWARD / CW /
    /// OUTWARD / CCW / OUTWARD2 mapping is used.
    fn neighbor_in_direction(&self, cell: u32, dir: Direction) -> Option<u32> {
        let (ring, pos) = self.ring_and_pos(cell);

        if ring == 0 {
            // Center cell: dir.0 is the index of the ring-1 neighbor.
            if self.rings > 1 && (dir.0 as u32) < self.cells_per_ring[1] {
                Some(self.cell_at(1, dir.0 as u32))
            } else {
                None
            }
        } else {
            self.non_center_neighbor(ring, pos, dir)
        }
    }

    /// Computes the outward child range for cell at (`ring`, `pos`).
    ///
    /// Returns `(out_start, out_end)` such that the outward neighbors in ring
    /// `ring + 1` occupy positions `[out_start, out_end)`. The range is derived
    /// as the inverse of the canonical inward mapping
    /// `parent(q) = q * cpr / outer_cpr`, guaranteeing bidirectional
    /// consistency.
    fn outward_range(&self, ring: u32, pos: u32) -> (u32, u32) {
        let cpr = self.cells_per_ring[ring as usize];
        let outer_cpr = self.cells_per_ring[(ring + 1) as usize];
        let out_start = (pos * outer_cpr).div_ceil(cpr);
        let out_end = ((pos + 1) * outer_cpr).div_ceil(cpr);
        (out_start, out_end)
    }

    /// Neighbor lookup for non-center cells.
    fn non_center_neighbor(&self, ring: u32, pos: u32, dir: Direction) -> Option<u32> {
        let cpr = self.cells_per_ring[ring as usize];

        match dir {
            d if d == CIRC_CW => {
                Some(self.cell_at(ring, (pos + 1) % cpr))
            }
            d if d == CIRC_CCW => {
                Some(self.cell_at(ring, (pos + cpr - 1) % cpr))
            }
            d if d == CIRC_INWARD => {
                if ring == 1 {
                    // Inward neighbor is the center cell.
                    Some(0)
                } else {
                    let inner_cpr = self.cells_per_ring[(ring - 1) as usize];
                    let inner_pos = pos * inner_cpr / cpr;
                    Some(self.cell_at(ring - 1, inner_pos))
                }
            }
            d if d == CIRC_OUTWARD || d == CIRC_OUTWARD2 => {
                if ring >= self.rings - 1 {
                    return None; // outermost ring has no outward neighbor
                }
                let (out_start, out_end) = self.outward_range(ring, pos);

                if d == CIRC_OUTWARD {
                    if out_start < out_end {
                        Some(self.cell_at(ring + 1, out_start))
                    } else {
                        None
                    }
                } else {
                    // CIRC_OUTWARD2: second outward neighbor (index 1).
                    if out_end - out_start >= 2 {
                        Some(self.cell_at(ring + 1, out_start + 1))
                    } else {
                        None
                    }
                }
            }
            _ => None,
        }
    }

    /// How many outward neighbors does the cell at (ring, pos) have?
    fn outward_count(&self, ring: u32, pos: u32) -> u32 {
        if ring >= self.rings - 1 {
            return 0;
        }
        let (out_start, out_end) = self.outward_range(ring, pos);
        out_end - out_start
    }

    /// Returns the direction from `cell` toward `neighbor`, if they are
    /// neighbors. Used internally for symmetric wall operations.
    fn direction_between(&self, cell: u32, neighbor: u32) -> Option<Direction> {
        let (r, p) = self.ring_and_pos(cell);

        if r == 0 {
            // Center: check if neighbor is in ring 1.
            let (nr, np) = self.ring_and_pos(neighbor);
            if nr == 1 {
                return Some(Direction(np as u8));
            }
            return None;
        }

        // Check each possible direction.
        for &dir in &[CIRC_INWARD, CIRC_CW, CIRC_OUTWARD, CIRC_CCW, CIRC_OUTWARD2] {
            if let Some(n) = self.non_center_neighbor(r, p, dir) {
                if n == neighbor {
                    return Some(dir);
                }
            }
        }
        None
    }

    /// The wall bitmask with all relevant bits set for a given cell.
    fn full_wall_mask(&self, cell: u32) -> u8 {
        let (ring, pos) = self.ring_and_pos(cell);

        if ring == 0 {
            // Center cell: one bit per ring-1 neighbor.
            if self.rings > 1 {
                let n = self.cells_per_ring[1] as u8;
                (1u8 << n) - 1
            } else {
                0
            }
        } else {
            // INWARD(0) + CW(1) + OUTWARD(2) + CCW(3), and possibly OUTWARD2(4).
            let base: u8 = 0b0000_1111; // bits 0..3
            let oc = self.outward_count(ring, pos);
            if oc >= 2 {
                base | (1 << 4) // add OUTWARD2 bit
            } else {
                base
            }
        }
    }
}

impl MazeGrid for CircularGrid {
    fn width(&self) -> u32 {
        if self.rings == 0 { 1 } else { 2 * self.rings - 1 }
    }

    fn height(&self) -> u32 {
        if self.rings == 0 { 1 } else { 2 * self.rings - 1 }
    }

    fn cell_count(&self) -> u32 {
        self.total_cells
    }

    fn topology(&self) -> Topology {
        Topology::Circular
    }

    fn directions(&self) -> &'static [Direction] {
        &CIRC_DIRECTIONS
    }

    fn opposite(&self, dir: Direction) -> Direction {
        match dir.0 {
            0 => CIRC_OUTWARD,  // INWARD -> OUTWARD
            1 => CIRC_CCW,      // CW -> CCW
            2 => CIRC_INWARD,   // OUTWARD -> INWARD
            3 => CIRC_CW,       // CCW -> CW
            4 => CIRC_INWARD,   // OUTWARD2 -> INWARD
            _ => dir,
        }
    }

    fn wall_bit(&self, dir: Direction) -> u8 {
        1 << dir.0
    }

    fn heuristic_distance(&self, from: u32, to: u32) -> u32 {
        let (r1, _) = self.ring_and_pos(from);
        let (r2, _) = self.ring_and_pos(to);
        r1.abs_diff(r2)
    }

    fn cell_position(&self, cell: u32) -> (f64, f64) {
        let (ring, pos) = self.ring_and_pos(cell);
        if ring == 0 {
            return (0.0, 0.0);
        }
        let cpr = self.cells_per_ring[ring as usize] as f64;
        let angle = 2.0 * std::f64::consts::PI * (pos as f64) / cpr;
        let r = ring as f64;
        (r * angle.cos(), r * angle.sin())
    }

    fn wall_bits(&self, cell: u32) -> u8 {
        self.walls[cell as usize]
    }

    fn wall_bits_ptr(&self) -> *const u8 {
        self.walls.as_ptr()
    }

    fn neighbors(&self, cell: u32) -> Vec<(u32, Direction)> {
        let (ring, pos) = self.ring_and_pos(cell);

        if ring == 0 {
            // Center cell: one neighbor per ring-1 cell.
            if self.rings <= 1 {
                return Vec::new();
            }
            let n = self.cells_per_ring[1];
            let mut result = Vec::with_capacity(n as usize);
            for p in 0..n {
                result.push((self.cell_at(1, p), Direction(p as u8)));
            }
            return result;
        }

        // Non-center cell: up to 5 neighbors.
        let mut result = Vec::with_capacity(5);

        // INWARD: always present for ring >= 1.
        if let Some(n) = self.non_center_neighbor(ring, pos, CIRC_INWARD) {
            result.push((n, CIRC_INWARD));
        }
        // CW
        if let Some(n) = self.non_center_neighbor(ring, pos, CIRC_CW) {
            result.push((n, CIRC_CW));
        }
        // OUTWARD
        if let Some(n) = self.non_center_neighbor(ring, pos, CIRC_OUTWARD) {
            result.push((n, CIRC_OUTWARD));
        }
        // CCW
        if let Some(n) = self.non_center_neighbor(ring, pos, CIRC_CCW) {
            result.push((n, CIRC_CCW));
        }
        // OUTWARD2
        if let Some(n) = self.non_center_neighbor(ring, pos, CIRC_OUTWARD2) {
            result.push((n, CIRC_OUTWARD2));
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
            // Find the reverse direction from the neighbor back to this cell.
            if let Some(rev_dir) = self.direction_between(neighbor, cell) {
                let rev_bit = self.wall_bit(rev_dir);
                self.walls[neighbor as usize] &= !rev_bit;
            }
        }
    }

    fn add_wall(&mut self, cell: u32, dir: Direction) {
        let bit = self.wall_bit(dir);
        self.walls[cell as usize] |= bit;

        if let Some(neighbor) = self.neighbor_in_direction(cell, dir) {
            if let Some(rev_dir) = self.direction_between(neighbor, cell) {
                let rev_bit = self.wall_bit(rev_dir);
                self.walls[neighbor as usize] |= rev_bit;
            }
        }
    }

    fn fill_walls(&mut self) {
        for cell in 0..self.total_cells {
            self.walls[cell as usize] = self.full_wall_mask(cell);
        }
    }

    fn clear_walls(&mut self) {
        self.walls.fill(0);
    }

    fn cell_index(&self, col: u32, row: u32) -> u32 {
        // row = ring, col = position within ring.
        self.ring_offset[row as usize] + col
    }

    fn cell_coords(&self, index: u32) -> (u32, u32) {
        let (ring, pos) = self.ring_and_pos(index);
        (pos, ring)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constructor_cell_count() {
        // Ring 0: 1, Ring 1: 6, Ring 2: 12, Ring 3: 18 => 1+6+12+18 = 37
        let grid = CircularGrid::new(4);
        assert_eq!(grid.cell_count(), 37);
    }

    #[test]
    fn test_single_ring() {
        let grid = CircularGrid::new(1);
        assert_eq!(grid.cell_count(), 1);
        assert!(grid.neighbors(0).is_empty());
    }

    #[test]
    fn test_cell_coords_roundtrip() {
        let grid = CircularGrid::new(5);
        for cell in 0..grid.cell_count() {
            let (col, row) = grid.cell_coords(cell);
            let reconstructed = grid.cell_index(col, row);
            assert_eq!(
                reconstructed, cell,
                "roundtrip failed for cell {cell}: coords=({col},{row}), reconstructed={reconstructed}"
            );
        }
    }

    #[test]
    fn test_center_has_correct_neighbor_count() {
        let grid = CircularGrid::new(3);
        let neighbors = grid.neighbors(0);
        // Center cell should have 6 outward neighbors (all cells in ring 1).
        assert_eq!(neighbors.len(), 6);
        // Each neighbor should be in ring 1.
        for &(n, _) in &neighbors {
            let (ring, _) = grid.ring_and_pos(n);
            assert_eq!(ring, 1);
        }
    }

    #[test]
    fn test_ring1_cells_have_correct_neighbors() {
        let grid = CircularGrid::new(3);
        // Ring 1 has 6 cells. Each should have:
        // - 1 inward neighbor (center)
        // - 1 CW neighbor
        // - 1 CCW neighbor
        // - outward neighbors (ring 2 has 12 cells, so each ring-1 cell has 2 outward)
        for p in 0..6u32 {
            let cell = grid.cell_at(1, p);
            let neighbors = grid.neighbors(cell);
            let dirs: Vec<Direction> = neighbors.iter().map(|&(_, d)| d).collect();

            assert!(dirs.contains(&CIRC_INWARD), "cell {cell} missing INWARD");
            assert!(dirs.contains(&CIRC_CW), "cell {cell} missing CW");
            assert!(dirs.contains(&CIRC_CCW), "cell {cell} missing CCW");
            assert!(dirs.contains(&CIRC_OUTWARD), "cell {cell} missing OUTWARD");
            assert!(dirs.contains(&CIRC_OUTWARD2), "cell {cell} missing OUTWARD2");
            assert_eq!(neighbors.len(), 5, "ring-1 cell {cell} should have 5 neighbors");
        }
    }

    #[test]
    fn test_remove_wall_symmetric() {
        let mut grid = CircularGrid::new(4);

        // Remove wall between center (cell 0) and first ring-1 cell (cell 1).
        let center = 0u32;
        let ring1_cell = grid.cell_at(1, 0);
        let center_dir = Direction(0); // center's direction toward ring1 pos 0

        assert!(grid.has_wall(center, center_dir));
        assert!(grid.has_wall(ring1_cell, CIRC_INWARD));

        grid.remove_wall(center, center_dir);

        assert!(!grid.has_wall(center, center_dir));
        assert!(!grid.has_wall(ring1_cell, CIRC_INWARD));
    }

    #[test]
    fn test_remove_wall_cw_ccw_symmetric() {
        let mut grid = CircularGrid::new(3);

        let cell_a = grid.cell_at(1, 0);
        let cell_b = grid.cell_at(1, 1);

        assert!(grid.has_wall(cell_a, CIRC_CW));
        assert!(grid.has_wall(cell_b, CIRC_CCW));

        grid.remove_wall(cell_a, CIRC_CW);

        assert!(!grid.has_wall(cell_a, CIRC_CW));
        assert!(!grid.has_wall(cell_b, CIRC_CCW));
    }

    #[test]
    fn test_fill_walls_sets_all() {
        let mut grid = CircularGrid::new(4);
        grid.clear_walls();
        grid.fill_walls();

        // Every cell should have all its relevant wall bits set.
        for cell in 0..grid.cell_count() {
            let expected = grid.full_wall_mask(cell);
            assert_eq!(
                grid.wall_bits(cell),
                expected,
                "cell {cell} wall bits mismatch after fill_walls"
            );
        }
    }

    #[test]
    fn test_clear_walls_removes_all() {
        let mut grid = CircularGrid::new(4);
        grid.clear_walls();

        for cell in 0..grid.cell_count() {
            assert_eq!(grid.wall_bits(cell), 0, "cell {cell} should have no walls");
        }
    }

    #[test]
    fn test_heuristic_distance_admissible() {
        let grid = CircularGrid::new(5);
        // Same cell => 0.
        assert_eq!(grid.heuristic_distance(0, 0), 0);

        // Center to any ring-1 cell => 1.
        let ring1 = grid.cell_at(1, 0);
        assert_eq!(grid.heuristic_distance(0, ring1), 1);

        // Ring 1 to ring 3 => |1-3| = 2.
        let ring3 = grid.cell_at(3, 0);
        assert_eq!(grid.heuristic_distance(ring1, ring3), 2);

        // Cells in the same ring => 0 (ring difference).
        let a = grid.cell_at(2, 0);
        let b = grid.cell_at(2, 5);
        assert_eq!(grid.heuristic_distance(a, b), 0);
    }

    #[test]
    fn test_cell_position_center_is_origin() {
        let grid = CircularGrid::new(3);
        let (x, y) = grid.cell_position(0);
        assert!((x).abs() < 1e-9);
        assert!((y).abs() < 1e-9);
    }

    #[test]
    fn test_cell_position_ring1() {
        let grid = CircularGrid::new(3);
        // Ring 1, position 0: angle = 0, so (1.0, 0.0).
        let cell = grid.cell_at(1, 0);
        let (x, y) = grid.cell_position(cell);
        assert!((x - 1.0).abs() < 1e-9, "x should be 1.0, got {x}");
        assert!(y.abs() < 1e-9, "y should be 0.0, got {y}");
    }

    #[test]
    fn test_add_wall_symmetric() {
        let mut grid = CircularGrid::new(3);
        grid.clear_walls();

        let cell_a = grid.cell_at(1, 2);
        let cell_b = grid.cell_at(1, 3);

        grid.add_wall(cell_a, CIRC_CW);
        assert!(grid.has_wall(cell_a, CIRC_CW));
        assert!(grid.has_wall(cell_b, CIRC_CCW));
    }

    #[test]
    fn test_outward_inward_symmetry() {
        // Removing an outward wall from ring 1 should clear the inward wall
        // on the corresponding ring-2 cell.
        let mut grid = CircularGrid::new(4);

        let inner = grid.cell_at(1, 0);
        let neighbors = grid.neighbors(inner);
        let outward_neighbors: Vec<(u32, Direction)> = neighbors
            .iter()
            .filter(|(_, d)| *d == CIRC_OUTWARD || *d == CIRC_OUTWARD2)
            .copied()
            .collect();

        assert!(!outward_neighbors.is_empty());

        let (outer, out_dir) = outward_neighbors[0];
        assert!(grid.has_wall(inner, out_dir));
        assert!(grid.has_wall(outer, CIRC_INWARD));

        grid.remove_wall(inner, out_dir);

        assert!(!grid.has_wall(inner, out_dir));
        assert!(!grid.has_wall(outer, CIRC_INWARD));
    }

    #[test]
    fn test_neighbors_bidirectional() {
        // For every cell A, if B is a neighbor of A via direction d, then A must
        // be a neighbor of B via some direction d'.
        let grid = CircularGrid::new(5);
        for cell in 0..grid.cell_count() {
            for &(neighbor, dir) in &grid.neighbors(cell) {
                let reverse = grid.neighbors(neighbor);
                let found = reverse.iter().any(|&(n, _)| n == cell);
                assert!(
                    found,
                    "cell {cell} -> {neighbor} via {dir:?}, but {neighbor} does not list {cell} as neighbor"
                );
            }
        }
    }

    #[test]
    fn test_topology() {
        let grid = CircularGrid::new(3);
        assert_eq!(grid.topology(), Topology::Circular);
    }

    #[test]
    fn test_width_height() {
        let grid = CircularGrid::new(5);
        assert_eq!(grid.width(), 9);
        assert_eq!(grid.height(), 9);
    }

    #[test]
    fn test_opposite_directions() {
        let grid = CircularGrid::new(2);
        assert_eq!(grid.opposite(CIRC_INWARD), CIRC_OUTWARD);
        assert_eq!(grid.opposite(CIRC_OUTWARD), CIRC_INWARD);
        assert_eq!(grid.opposite(CIRC_CW), CIRC_CCW);
        assert_eq!(grid.opposite(CIRC_CCW), CIRC_CW);
        assert_eq!(grid.opposite(CIRC_OUTWARD2), CIRC_INWARD);
    }

    #[test]
    fn test_outermost_ring_no_outward() {
        let grid = CircularGrid::new(3);
        // Ring 2 (outermost) cells should have no outward neighbors.
        let cpr = grid.cells_per_ring[2];
        for p in 0..cpr {
            let cell = grid.cell_at(2, p);
            let neighbors = grid.neighbors(cell);
            for &(_, dir) in &neighbors {
                assert!(
                    dir != CIRC_OUTWARD && dir != CIRC_OUTWARD2,
                    "outermost ring cell {cell} should have no outward neighbor"
                );
            }
        }
    }
}
