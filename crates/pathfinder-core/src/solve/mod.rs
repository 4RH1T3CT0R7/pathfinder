mod bfs;
mod dfs_solver;
mod astar;
mod dijkstra;
mod greedy_bfs;
mod wall_follower;
mod tremaux;
mod dead_end_filling;

pub use bfs::BfsSolver;
pub use dfs_solver::DfsSolver;
pub use astar::AStarSolver;
pub use dijkstra::DijkstraSolver;
pub use greedy_bfs::GreedyBfsSolver;
pub use wall_follower::WallFollowerSolver;
pub use tremaux::TremauxSolver;
pub use dead_end_filling::DeadEndFillingSolver;

#[derive(Debug, Clone)]
pub struct SolveStep {
    pub cell: u32,
    pub action: SolveAction,
}

#[derive(Debug, Clone)]
pub enum SolveAction {
    Visit,
    AddToFrontier,
    MarkPath,
    FoundGoal,
}

pub trait SteppableSolver {
    /// Perform one step of the solving algorithm.
    /// Returns `Some(step)` if work was done, `None` if done (found or impossible).
    fn step(&mut self, maze: &dyn crate::maze::MazeGrid) -> Option<SolveStep>;

    /// Returns the solution path once found (cell indices from start to end).
    fn path(&self) -> Option<&[u32]>;

    /// Reset the solver for a new run.
    fn reset(&mut self, start: u32, end: u32);

    /// Returns true if solving is complete.
    fn is_done(&self) -> bool;
}
