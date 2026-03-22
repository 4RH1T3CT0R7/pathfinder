mod grid;
mod hex;
mod triangle;
mod circular;

pub use grid::RectGrid;
pub use hex::HexGrid;
pub use triangle::TriangleGrid;
pub use circular::CircularGrid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Direction(pub u8);

impl Direction {
    pub const NORTH: Direction = Direction(0);
    pub const EAST: Direction = Direction(1);
    pub const SOUTH: Direction = Direction(2);
    pub const WEST: Direction = Direction(3);

    /// Returns the static list of rectangular grid directions, for convenience.
    /// Prefer using `MazeGrid::directions()` when a grid reference is available.
    pub fn all_rect() -> &'static [Direction] {
        &[Direction::NORTH, Direction::EAST, Direction::SOUTH, Direction::WEST]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Topology {
    Rectangular,
    Hexagonal,
    Triangular,
    Circular,
}

/// Cell visit state packed into bits [4:6] of cell state byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CellState {
    Unvisited = 0,
    InFrontier = 1,
    Visited = 2,
    OnPath = 3,
    OnSolution = 4,
    Backtracked = 5,
}

impl CellState {
    pub fn from_bits(bits: u8) -> Self {
        match (bits >> 4) & 0x07 {
            0 => CellState::Unvisited,
            1 => CellState::InFrontier,
            2 => CellState::Visited,
            3 => CellState::OnPath,
            4 => CellState::OnSolution,
            5 => CellState::Backtracked,
            _ => CellState::Unvisited,
        }
    }

    pub fn to_bits(self) -> u8 {
        (self as u8) << 4
    }
}

pub trait MazeGrid {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn cell_count(&self) -> u32;
    fn topology(&self) -> Topology;

    /// Returns the canonical set of directions for this topology.
    fn directions(&self) -> &'static [Direction];

    /// Returns the opposite direction for this topology.
    fn opposite(&self, dir: Direction) -> Direction;

    /// Returns the wall bitmask bit for a given direction.
    fn wall_bit(&self, dir: Direction) -> u8;

    /// Returns the heuristic distance between two cells (for A*, GreedyBFS).
    fn heuristic_distance(&self, from: u32, to: u32) -> u32;

    /// Returns the (x, y) position of a cell for rendering purposes.
    fn cell_position(&self, cell: u32) -> (f64, f64);

    /// Returns the raw wall bitmask for a cell.
    fn wall_bits(&self, cell: u32) -> u8;

    /// Pointer to the raw wall data for direct WASM memory access.
    fn wall_bits_ptr(&self) -> *const u8;

    /// Returns the list of (neighbor_cell_index, direction_to_neighbor).
    fn neighbors(&self, cell: u32) -> Vec<(u32, Direction)>;

    fn has_wall(&self, cell: u32, dir: Direction) -> bool;
    fn remove_wall(&mut self, cell: u32, dir: Direction);
    fn add_wall(&mut self, cell: u32, dir: Direction);

    /// Initialize all walls (fully walled maze).
    fn fill_walls(&mut self);

    /// Remove all walls (open maze).
    fn clear_walls(&mut self);

    fn cell_index(&self, col: u32, row: u32) -> u32;
    fn cell_coords(&self, index: u32) -> (u32, u32);
}
