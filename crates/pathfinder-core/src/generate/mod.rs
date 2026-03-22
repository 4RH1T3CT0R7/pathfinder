mod dfs;
mod kruskal;
mod prim;
mod eller;
mod wilson;
mod growing_tree;
mod binary_tree;
mod sidewinder;
mod aldous_broder;
mod hunt_and_kill;

pub use dfs::DfsGenerator;
pub use kruskal::KruskalGenerator;
pub use prim::PrimGenerator;
pub use eller::EllerGenerator;
pub use wilson::WilsonGenerator;
pub use growing_tree::{GrowingTreeGenerator, SelectionStrategy};
pub use binary_tree::BinaryTreeGenerator;
pub use sidewinder::SidewinderGenerator;
pub use aldous_broder::AldousBroderGenerator;
pub use hunt_and_kill::HuntAndKillGenerator;

use crate::maze::{Direction, MazeGrid};
use crate::rng::Xoshiro256;

#[derive(Debug, Clone)]
pub struct GenStep {
    pub cell: u32,
    pub action: GenAction,
}

#[derive(Debug, Clone)]
pub enum GenAction {
    Visit,
    RemoveWall(Direction),
    Backtrack,
}

pub trait SteppableGenerator {
    /// Perform one step of the generation algorithm.
    /// Returns `Some(step)` if work was done, `None` if generation is complete.
    fn step(&mut self, maze: &mut dyn MazeGrid, rng: &mut Xoshiro256) -> Option<GenStep>;

    /// Reset the generator for a new run.
    fn reset(&mut self, start_cell: u32);

    /// Returns true if generation is complete.
    fn is_done(&self) -> bool;
}
