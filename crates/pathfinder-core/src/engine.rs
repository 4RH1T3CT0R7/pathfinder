use wasm_bindgen::prelude::*;

use crate::maze::{Direction, MazeGrid, RectGrid, HexGrid, TriangleGrid, CircularGrid};
use crate::generate::{
    GenAction, SteppableGenerator, DfsGenerator, KruskalGenerator, PrimGenerator, EllerGenerator,
    WilsonGenerator, GrowingTreeGenerator, BinaryTreeGenerator,
    SidewinderGenerator, AldousBroderGenerator, HuntAndKillGenerator,
};
use crate::solve::{
    SolveAction, SteppableSolver, BfsSolver, DfsSolver, AStarSolver, DijkstraSolver,
    GreedyBfsSolver, WallFollowerSolver, TremauxSolver, DeadEndFillingSolver,
};
use crate::rng::Xoshiro256;
use crate::metrics::Metrics;

/// Visual state constants for per-cell animation tracking.
/// These values are exposed to JS as raw `u8` bytes in `cell_states`.
const CELL_UNVISITED: u8 = 0;
const CELL_FRONTIER: u8 = 1;
const CELL_VISITED: u8 = 2;
const CELL_ACTIVE: u8 = 3;
const CELL_SOLUTION: u8 = 4;
const CELL_BACKTRACKED: u8 = 5;

enum Generator {
    Dfs(DfsGenerator),
    Kruskal(KruskalGenerator),
    Prim(PrimGenerator),
    Eller(EllerGenerator),
    Wilson(WilsonGenerator),
    GrowingTree(GrowingTreeGenerator),
    BinaryTree(BinaryTreeGenerator),
    Sidewinder(SidewinderGenerator),
    AldousBroder(AldousBroderGenerator),
    HuntAndKill(HuntAndKillGenerator),
}

#[allow(dead_code)]
impl Generator {
    fn step(&mut self, maze: &mut dyn MazeGrid, rng: &mut Xoshiro256) -> Option<crate::generate::GenStep> {
        match self {
            Generator::Dfs(g) => g.step(maze, rng),
            Generator::Kruskal(g) => g.step(maze, rng),
            Generator::Prim(g) => g.step(maze, rng),
            Generator::Eller(g) => g.step(maze, rng),
            Generator::Wilson(g) => g.step(maze, rng),
            Generator::GrowingTree(g) => g.step(maze, rng),
            Generator::BinaryTree(g) => g.step(maze, rng),
            Generator::Sidewinder(g) => g.step(maze, rng),
            Generator::AldousBroder(g) => g.step(maze, rng),
            Generator::HuntAndKill(g) => g.step(maze, rng),
        }
    }

    fn is_done(&self) -> bool {
        match self {
            Generator::Dfs(g) => g.is_done(),
            Generator::Kruskal(g) => g.is_done(),
            Generator::Prim(g) => g.is_done(),
            Generator::Eller(g) => g.is_done(),
            Generator::Wilson(g) => g.is_done(),
            Generator::GrowingTree(g) => g.is_done(),
            Generator::BinaryTree(g) => g.is_done(),
            Generator::Sidewinder(g) => g.is_done(),
            Generator::AldousBroder(g) => g.is_done(),
            Generator::HuntAndKill(g) => g.is_done(),
        }
    }
}

enum Solver {
    Bfs(BfsSolver),
    Dfs(DfsSolver),
    AStar(AStarSolver),
    Dijkstra(DijkstraSolver),
    GreedyBfs(GreedyBfsSolver),
    WallFollower(WallFollowerSolver),
    Tremaux(TremauxSolver),
    DeadEndFilling(DeadEndFillingSolver),
}

#[allow(dead_code)]
impl Solver {
    fn step(&mut self, maze: &dyn MazeGrid) -> Option<crate::solve::SolveStep> {
        match self {
            Solver::Bfs(s) => s.step(maze),
            Solver::Dfs(s) => s.step(maze),
            Solver::AStar(s) => s.step(maze),
            Solver::Dijkstra(s) => s.step(maze),
            Solver::GreedyBfs(s) => s.step(maze),
            Solver::WallFollower(s) => s.step(maze),
            Solver::Tremaux(s) => s.step(maze),
            Solver::DeadEndFilling(s) => s.step(maze),
        }
    }

    fn path(&self) -> Option<&[u32]> {
        match self {
            Solver::Bfs(s) => s.path(),
            Solver::Dfs(s) => s.path(),
            Solver::AStar(s) => s.path(),
            Solver::Dijkstra(s) => s.path(),
            Solver::GreedyBfs(s) => s.path(),
            Solver::WallFollower(s) => s.path(),
            Solver::Tremaux(s) => s.path(),
            Solver::DeadEndFilling(s) => s.path(),
        }
    }

    fn is_done(&self) -> bool {
        match self {
            Solver::Bfs(s) => s.is_done(),
            Solver::Dfs(s) => s.is_done(),
            Solver::AStar(s) => s.is_done(),
            Solver::Dijkstra(s) => s.is_done(),
            Solver::GreedyBfs(s) => s.is_done(),
            Solver::WallFollower(s) => s.is_done(),
            Solver::Tremaux(s) => s.is_done(),
            Solver::DeadEndFilling(s) => s.is_done(),
        }
    }
}

/// Grid topology abstraction supporting rectangular, hexagonal, triangular,
/// and circular maze topologies.
enum GridKind {
    Rect(RectGrid),
    Hex(HexGrid),
    Triangle(TriangleGrid),
    Circular(CircularGrid),
}

impl GridKind {
    fn as_grid(&self) -> &dyn MazeGrid {
        match self {
            GridKind::Rect(g) => g,
            GridKind::Hex(g) => g,
            GridKind::Triangle(g) => g,
            GridKind::Circular(g) => g,
        }
    }

    fn as_grid_mut(&mut self) -> &mut dyn MazeGrid {
        match self {
            GridKind::Rect(g) => g,
            GridKind::Hex(g) => g,
            GridKind::Triangle(g) => g,
            GridKind::Circular(g) => g,
        }
    }
}

#[wasm_bindgen]
pub struct MazeEngine {
    grid: GridKind,
    generator: Option<Generator>,
    solver: Option<Solver>,
    rng: Xoshiro256,
    metrics: Metrics,
    gen_done: bool,
    solve_done: bool,
    /// Per-cell visual state for progressive animation.
    /// Values: 0=unvisited, 1=frontier, 2=visited, 3=active, 4=solution, 5=backtracked.
    cell_states: Vec<u8>,
    /// Step number when each cell was first visited (for heatmap rendering).
    visit_order: Vec<u32>,
    /// Monotonically increasing counter for recording visit order.
    step_counter: u32,
    /// Topology identifier: 0=Rect, 1=Hex, 2=Triangle, 3=Circular.
    topology_id: u8,
    /// Pre-computed cell positions as flat [x0, y0, x1, y1, ...] for JS rendering.
    cell_positions: Vec<f64>,
}

#[wasm_bindgen]
impl MazeEngine {
    /// Create a new maze engine.
    ///
    /// `topology`: 0=Rect, 1=Hex, 2=Triangle, 3=Circular.
    /// For topologies 0-2, `width` and `height` specify grid dimensions.
    /// For topology 3 (Circular), `width` is used as the `rings` parameter.
    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32, seed_hi: u32, seed_lo: u32, topology: u8) -> Self {
        let seed = ((seed_hi as u64) << 32) | (seed_lo as u64);
        let grid = match topology {
            1 => GridKind::Hex(HexGrid::new(width, height)),
            2 => GridKind::Triangle(TriangleGrid::new(width, height)),
            3 => GridKind::Circular(CircularGrid::new(width.max(1))),
            _ => GridKind::Rect(RectGrid::new(width, height)),
        };
        let count = grid.as_grid().cell_count() as usize;

        // Pre-compute cell positions for JS rendering
        let mut cell_positions = Vec::with_capacity(count * 2);
        for i in 0..count {
            let (x, y) = grid.as_grid().cell_position(i as u32);
            cell_positions.push(x);
            cell_positions.push(y);
        }

        MazeEngine {
            grid,
            generator: None,
            solver: None,
            rng: Xoshiro256::new(seed),
            metrics: Metrics::default(),
            gen_done: false,
            solve_done: false,
            cell_states: vec![CELL_UNVISITED; count],
            visit_order: vec![0; count],
            step_counter: 0,
            topology_id: topology.min(3),
            cell_positions,
        }
    }

    pub fn width(&self) -> u32 {
        self.grid.as_grid().width()
    }

    pub fn height(&self) -> u32 {
        self.grid.as_grid().height()
    }

    pub fn cell_count(&self) -> u32 {
        self.grid.as_grid().cell_count()
    }

    /// Initialize a generator by algorithm id.
    /// 0=DFS, 1=Kruskal, 2=Prim, 3=Eller, 4=Wilson,
    /// 5=GrowingTree, 6=BinaryTree, 7=Sidewinder, 8=AldousBroder, 9=HuntAndKill
    ///
    /// For non-rectangular topologies, rect-only generators (BinaryTree=6,
    /// Sidewinder=7, Eller=3) are not supported and fall back to DFS.
    pub fn init_generator(&mut self, algo: u8, start_cell: u32) {
        self.grid.as_grid_mut().fill_walls();
        let cc = self.grid.as_grid().cell_count();
        let w = self.grid.as_grid().width();
        let h = self.grid.as_grid().height();
        let is_rect = self.topology_id == 0;

        // Rect-only generators fall back to DFS on non-rect topologies
        let effective_algo = if !is_rect && matches!(algo, 3 | 6 | 7) {
            0 // DFS
        } else {
            algo
        };

        let gen = match effective_algo {
            0 => Generator::Dfs(DfsGenerator::new(cc, start_cell)),
            1 => Generator::Kruskal(KruskalGenerator::new(cc, start_cell)),
            2 => Generator::Prim(PrimGenerator::new(cc, start_cell)),
            3 => Generator::Eller(EllerGenerator::new(cc, start_cell)),
            4 => Generator::Wilson(WilsonGenerator::new(cc, start_cell)),
            5 => Generator::GrowingTree(GrowingTreeGenerator::new(cc, start_cell)),
            6 => Generator::BinaryTree(BinaryTreeGenerator::new(cc, start_cell)),
            7 => Generator::Sidewinder(SidewinderGenerator::new(w, h)),
            8 => Generator::AldousBroder(AldousBroderGenerator::new(cc, start_cell)),
            9 => Generator::HuntAndKill(HuntAndKillGenerator::new(cc, start_cell)),
            _ => Generator::Dfs(DfsGenerator::new(cc, start_cell)),
        };
        self.generator = Some(gen);
        self.gen_done = false;
        self.metrics = Metrics::default();
        self.cell_states.fill(CELL_UNVISITED);
        self.visit_order.fill(0);
        self.step_counter = 0;
    }

    /// Run N generation steps. Returns the number of steps actually performed.
    pub fn step_generate(&mut self, n: u32) -> u32 {
        let gen = match &mut self.generator {
            Some(g) => g,
            None => return 0,
        };

        let mut performed = 0;
        for _ in 0..n {
            match gen.step(self.grid.as_grid_mut(), &mut self.rng) {
                Some(step) => {
                    performed += 1;
                    self.metrics.steps_taken += 1;
                    self.step_counter += 1;

                    let idx = step.cell as usize;
                    match step.action {
                        GenAction::Visit => {
                            self.cell_states[idx] = CELL_VISITED;
                        }
                        GenAction::RemoveWall(_) => {
                            self.cell_states[idx] = CELL_ACTIVE;
                            self.visit_order[idx] = self.step_counter;
                        }
                        GenAction::Backtrack => {
                            self.cell_states[idx] = CELL_BACKTRACKED;
                        }
                    }
                }
                None => {
                    self.gen_done = true;
                    break;
                }
            }
        }
        performed
    }

    /// Initialize a solver by algorithm id.
    /// 0=BFS, 1=DFS, 2=A*, 3=Dijkstra, 4=GreedyBFS,
    /// 5=WallFollower, 6=Tremaux, 7=DeadEndFilling
    pub fn init_solver(&mut self, algo: u8, start: u32, end: u32) {
        let cc = self.grid.as_grid().cell_count();
        let solver = match algo {
            0 => Solver::Bfs(BfsSolver::new(cc, start, end)),
            1 => Solver::Dfs(DfsSolver::new(cc, start, end)),
            2 => Solver::AStar(AStarSolver::new(self.grid.as_grid(), start, end)),
            3 => Solver::Dijkstra(DijkstraSolver::new(cc, start, end)),
            4 => Solver::GreedyBfs(GreedyBfsSolver::new(self.grid.as_grid(), start, end)),
            5 => Solver::WallFollower(WallFollowerSolver::new(cc, start, end)),
            6 => Solver::Tremaux(TremauxSolver::new(cc, start, end)),
            7 => Solver::DeadEndFilling(DeadEndFillingSolver::new(cc, start, end)),
            _ => Solver::Bfs(BfsSolver::new(cc, start, end)),
        };
        self.solver = Some(solver);
        self.solve_done = false;
        self.metrics.steps_taken = 0;
        self.metrics.cells_visited = 0;
        self.metrics.path_length = 0;
        self.cell_states.fill(CELL_UNVISITED);
        self.visit_order.fill(0);
        self.step_counter = 0;
        self.cell_states[start as usize] = CELL_ACTIVE;
    }

    /// Run N solving steps. Returns the number of steps actually performed.
    pub fn step_solve(&mut self, n: u32) -> u32 {
        let solver = match &mut self.solver {
            Some(s) => s,
            None => return 0,
        };

        let mut performed = 0;
        for _ in 0..n {
            match solver.step(self.grid.as_grid()) {
                Some(step) => {
                    performed += 1;
                    self.metrics.steps_taken += 1;
                    self.metrics.cells_visited += 1;
                    self.step_counter += 1;

                    let idx = step.cell as usize;
                    match step.action {
                        SolveAction::Visit => {
                            self.cell_states[idx] = CELL_VISITED;
                            self.visit_order[idx] = self.step_counter;
                        }
                        SolveAction::AddToFrontier => {
                            self.cell_states[idx] = CELL_FRONTIER;
                        }
                        SolveAction::MarkPath => {
                            self.cell_states[idx] = CELL_SOLUTION;
                        }
                        SolveAction::FoundGoal => {
                            self.cell_states[idx] = CELL_SOLUTION;
                            // Mark the entire solution path now that it is available.
                            if let Some(path) = solver.path() {
                                for &cell in path {
                                    self.cell_states[cell as usize] = CELL_SOLUTION;
                                }
                            }
                        }
                    }
                }
                None => {
                    self.solve_done = true;
                    if let Some(path) = solver.path() {
                        self.metrics.path_length = path.len() as u32;
                        // Final pass: ensure solution path cells are marked.
                        for &cell in path {
                            self.cell_states[cell as usize] = CELL_SOLUTION;
                        }
                    }
                    break;
                }
            }
        }
        performed
    }

    pub fn is_generation_done(&self) -> bool {
        self.gen_done
    }

    pub fn is_solving_done(&self) -> bool {
        self.solve_done
    }

    pub fn cell_walls(&self, cell: u32) -> u8 {
        self.grid.as_grid().wall_bits(cell)
    }

    pub fn solution_path_json(&self) -> String {
        match &self.solver {
            Some(solver) => match solver.path() {
                Some(path) => serde_json::to_string(path).unwrap_or_else(|_| "[]".to_string()),
                None => "[]".to_string(),
            },
            None => "[]".to_string(),
        }
    }

    pub fn get_metrics_json(&self) -> String {
        serde_json::to_string(&self.metrics).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn walls_ptr(&self) -> *const u8 {
        self.grid.as_grid().wall_bits_ptr()
    }

    pub fn toggle_wall(&mut self, cell: u32, direction: u8) {
        let dir = Direction(direction);
        // Validate direction against max directions for current topology
        let max_dirs = self.directions_count();
        if direction >= max_dirs {
            return;
        }
        let grid = self.grid.as_grid_mut();
        if grid.has_wall(cell, dir) {
            grid.remove_wall(cell, dir);
        } else {
            grid.add_wall(cell, dir);
        }
    }

    /// Returns the topology identifier: 0=Rect, 1=Hex, 2=Triangle, 3=Circular.
    pub fn topology(&self) -> u8 {
        self.topology_id
    }

    /// Number of directions for the current topology.
    /// 4 for rect, 6 for hex, 3 for triangle, 4 for circular.
    pub fn directions_count(&self) -> u8 {
        self.grid.as_grid().directions().len() as u8
    }

    /// Pointer to the pre-computed cell positions buffer for zero-copy JS access.
    /// Layout: flat `[x0, y0, x1, y1, ...]` array of `f64` values.
    pub fn cell_positions_ptr(&self) -> *const f64 {
        self.cell_positions.as_ptr()
    }

    /// Number of `f64` values in the cell positions buffer (= cell_count * 2).
    pub fn cell_positions_len(&self) -> u32 {
        self.cell_positions.len() as u32
    }

    /// Pointer to the `cell_states` buffer for zero-copy JS access via WASM linear memory.
    /// The buffer contains `cell_count()` bytes, one per cell.
    pub fn cell_states_ptr(&self) -> *const u8 {
        self.cell_states.as_ptr()
    }

    /// Pointer to the `visit_order` buffer for heatmap rendering via WASM linear memory.
    /// The buffer contains `cell_count()` `u32` values (4 bytes each), one per cell.
    pub fn visit_order_ptr(&self) -> *const u32 {
        self.visit_order.as_ptr()
    }

    /// Get the visual state of a single cell (fallback for non-shared-memory access).
    /// Returns 0-5 matching the `CELL_*` constants.
    pub fn cell_state(&self, cell: u32) -> u8 {
        self.cell_states.get(cell as usize).copied().unwrap_or(CELL_UNVISITED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create an engine and fully generate a maze.
    fn engine_with_maze(w: u32, h: u32, seed: u64) -> MazeEngine {
        let seed_hi = (seed >> 32) as u32;
        let seed_lo = seed as u32;
        let mut engine = MazeEngine::new(w, h, seed_hi, seed_lo, 0);
        engine.init_generator(0, 0); // DFS from cell 0
        while !engine.is_generation_done() {
            engine.step_generate(100);
        }
        engine
    }

    #[test]
    fn test_cell_states_initialized_to_zero() {
        let engine = MazeEngine::new(5, 5, 0, 42, 0);
        assert_eq!(engine.cell_states.len(), 25);
        assert!(engine.cell_states.iter().all(|&s| s == CELL_UNVISITED));
        assert!(engine.visit_order.iter().all(|&v| v == 0));
    }

    #[test]
    fn test_init_generator_resets_cell_states() {
        let mut engine = MazeEngine::new(4, 4, 0, 1, 0);
        // Dirty the buffers
        engine.cell_states[0] = CELL_VISITED;
        engine.visit_order[0] = 99;
        engine.step_counter = 50;

        engine.init_generator(0, 0);

        assert!(engine.cell_states.iter().all(|&s| s == CELL_UNVISITED));
        assert!(engine.visit_order.iter().all(|&v| v == 0));
        assert_eq!(engine.step_counter, 0);
    }

    #[test]
    fn test_generation_populates_cell_states() {
        let engine = engine_with_maze(5, 5, 42);

        // After full generation every cell should have been touched at least once.
        // No cell should remain CELL_UNVISITED (DFS visits all cells).
        for i in 0..25 {
            assert_ne!(
                engine.cell_states[i], CELL_UNVISITED,
                "cell {} was never visited during generation",
                i
            );
        }
    }

    #[test]
    fn test_generation_records_visit_order() {
        let engine = engine_with_maze(5, 5, 42);

        // At least some cells should have a non-zero visit_order
        // (every RemoveWall action records it).
        let recorded = engine.visit_order.iter().filter(|&&v| v > 0).count();
        assert!(
            recorded > 0,
            "visit_order should have been recorded for at least some cells"
        );
    }

    #[test]
    fn test_init_solver_resets_and_marks_start() {
        let mut engine = engine_with_maze(5, 5, 42);

        let start = 0u32;
        let end = 24u32;
        engine.init_solver(0, start, end); // BFS

        // All cells should be unvisited except the start cell.
        assert_eq!(engine.cell_states[start as usize], CELL_ACTIVE);
        for i in 1..25 {
            assert_eq!(
                engine.cell_states[i], CELL_UNVISITED,
                "cell {} should be unvisited after init_solver",
                i
            );
        }
        assert_eq!(engine.step_counter, 0);
    }

    #[test]
    fn test_solve_marks_visited_and_solution() {
        let mut engine = engine_with_maze(5, 5, 42);

        let start = 0u32;
        let end = 24u32;
        engine.init_solver(0, start, end); // BFS

        while !engine.is_solving_done() {
            engine.step_solve(100);
        }

        // The start and end cells should be on the solution path.
        assert_eq!(engine.cell_states[start as usize], CELL_SOLUTION);
        assert_eq!(engine.cell_states[end as usize], CELL_SOLUTION);

        // At least some cells should be marked as visited (explored but not on solution path).
        let visited_count = engine
            .cell_states
            .iter()
            .filter(|&&s| s == CELL_VISITED)
            .count();
        assert!(
            visited_count > 0,
            "some cells should be marked as visited (explored) during solve"
        );

        // Verify visit_order was recorded for visited cells.
        let ordered_count = engine.visit_order.iter().filter(|&&v| v > 0).count();
        assert!(ordered_count > 0, "visit_order should be recorded during solve");
    }

    #[test]
    fn test_solution_path_cells_all_marked() {
        let mut engine = engine_with_maze(8, 8, 99);

        let start = 0u32;
        let end = 63u32;
        engine.init_solver(0, start, end);

        while !engine.is_solving_done() {
            engine.step_solve(500);
        }

        // Parse the solution path and verify every cell on it is marked CELL_SOLUTION.
        let path_json = engine.solution_path_json();
        let path: Vec<u32> = serde_json::from_str(&path_json).unwrap();
        assert!(!path.is_empty(), "solution path should not be empty");

        for &cell in &path {
            assert_eq!(
                engine.cell_states[cell as usize],
                CELL_SOLUTION,
                "cell {} is on solution path but not marked as CELL_SOLUTION",
                cell
            );
        }
    }

    #[test]
    fn test_cell_state_accessor_bounds() {
        let engine = MazeEngine::new(3, 3, 0, 1, 0);
        // In-bounds access
        assert_eq!(engine.cell_state(0), CELL_UNVISITED);
        assert_eq!(engine.cell_state(8), CELL_UNVISITED);
        // Out-of-bounds returns CELL_UNVISITED
        assert_eq!(engine.cell_state(100), CELL_UNVISITED);
        assert_eq!(engine.cell_state(u32::MAX), CELL_UNVISITED);
    }

    #[test]
    fn test_cell_states_ptr_not_null() {
        let engine = MazeEngine::new(4, 4, 0, 1, 0);
        assert!(!engine.cell_states_ptr().is_null());
        assert!(!engine.visit_order_ptr().is_null());
    }

    #[test]
    fn test_step_counter_increments() {
        let mut engine = MazeEngine::new(5, 5, 0, 42, 0);
        engine.init_generator(0, 0);

        let steps = engine.step_generate(10);
        assert_eq!(engine.step_counter, steps);

        engine.init_solver(0, 0, 24);
        let solve_steps = engine.step_solve(5);
        assert_eq!(engine.step_counter, solve_steps);
    }

    #[test]
    fn test_backtrack_marks_cell() {
        let mut engine = MazeEngine::new(5, 5, 0, 42, 0);
        engine.init_generator(0, 0); // DFS

        // Run generation to completion; DFS will backtrack.
        while !engine.is_generation_done() {
            engine.step_generate(1);
        }

        // At least some cells should be marked as backtracked.
        let backtracked = engine
            .cell_states
            .iter()
            .filter(|&&s| s == CELL_BACKTRACKED)
            .count();
        assert!(
            backtracked > 0,
            "DFS generation should produce at least one backtrack"
        );
    }

    #[test]
    fn test_hex_topology_generates_and_solves() {
        let mut engine = MazeEngine::new(6, 6, 0, 42, 1);
        assert_eq!(engine.topology(), 1);
        assert_eq!(engine.directions_count(), 6);
        assert!(engine.cell_positions_len() > 0);
        engine.init_generator(0, 0);
        while !engine.is_generation_done() {
            engine.step_generate(100);
        }
        let cc = engine.cell_count();
        engine.init_solver(0, 0, cc - 1);
        while !engine.is_solving_done() {
            engine.step_solve(100);
        }
        let path_json = engine.solution_path_json();
        let path: Vec<u32> = serde_json::from_str(&path_json).unwrap();
        assert!(!path.is_empty());
    }

    #[test]
    fn test_triangle_topology_generates_and_solves() {
        let mut engine = MazeEngine::new(8, 6, 0, 42, 2);
        assert_eq!(engine.topology(), 2);
        assert_eq!(engine.directions_count(), 3);
        engine.init_generator(0, 0);
        while !engine.is_generation_done() {
            engine.step_generate(100);
        }
        let cc = engine.cell_count();
        engine.init_solver(0, 0, cc - 1);
        while !engine.is_solving_done() {
            engine.step_solve(100);
        }
        let path_json = engine.solution_path_json();
        let path: Vec<u32> = serde_json::from_str(&path_json).unwrap();
        assert!(!path.is_empty());
    }

    #[test]
    fn test_circular_topology_generates_and_solves() {
        let mut engine = MazeEngine::new(5, 1, 0, 42, 3);
        assert_eq!(engine.topology(), 3);
        assert_eq!(engine.directions_count(), 4);
        engine.init_generator(0, 0);
        while !engine.is_generation_done() {
            engine.step_generate(200);
        }
        let cc = engine.cell_count();
        engine.init_solver(0, 0, cc - 1);
        while !engine.is_solving_done() {
            engine.step_solve(200);
        }
        let path_json = engine.solution_path_json();
        let path: Vec<u32> = serde_json::from_str(&path_json).unwrap();
        assert!(!path.is_empty());
    }

    #[test]
    fn test_rect_only_generators_fallback_on_hex() {
        // Eller (3), BinaryTree (6), Sidewinder (7) should fall back to DFS
        let mut engine = MazeEngine::new(5, 5, 0, 42, 1);
        engine.init_generator(6, 0); // BinaryTree -> should fallback to DFS
        while !engine.is_generation_done() {
            engine.step_generate(200);
        }
        // If it completes without panic, the fallback worked
        assert!(engine.is_generation_done());
    }

    #[test]
    fn test_cell_positions_populated() {
        let engine = MazeEngine::new(4, 4, 0, 1, 0);
        let cc = engine.cell_count();
        assert_eq!(engine.cell_positions_len(), cc * 2);
        assert!(!engine.cell_positions_ptr().is_null());
    }
}
