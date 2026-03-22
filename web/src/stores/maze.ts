import { createSignal } from 'solid-js';

export type GeneratorAlgo = 'dfs' | 'kruskal' | 'prim' | 'eller' | 'wilson' | 'growing_tree' | 'binary_tree' | 'sidewinder' | 'aldous_broder' | 'hunt_and_kill';
export type SolverAlgo = 'bfs' | 'dfs' | 'astar' | 'dijkstra' | 'greedy_bfs' | 'wall_follower' | 'tremaux' | 'dead_end_filling';

export type Topology = 'rectangular' | 'hexagonal' | 'triangular' | 'circular';
export const TOPOLOGY_ID: Record<Topology, number> = { rectangular: 0, hexagonal: 1, triangular: 2, circular: 3 };

/** Generators that only work on rectangular grids. */
export const RECT_ONLY_GENERATORS: GeneratorAlgo[] = ['binary_tree', 'sidewinder', 'eller'];

export const ALL_SOLVER_ALGOS: SolverAlgo[] = [
  'bfs', 'dfs', 'astar', 'dijkstra', 'greedy_bfs', 'wall_follower', 'tremaux', 'dead_end_filling',
];

export const SOLVER_ALGO_NAMES: Record<SolverAlgo, string> = {
  bfs: 'BFS (Breadth-First)',
  dfs: 'DFS (Depth-First)',
  astar: 'A* Search',
  dijkstra: 'Dijkstra',
  greedy_bfs: 'Greedy Best-First',
  wall_follower: 'Wall Follower',
  tremaux: 'Tremaux',
  dead_end_filling: 'Dead-End Filling',
};

export const GENERATOR_ALGO_ID: Record<GeneratorAlgo, number> = {
  dfs: 0, kruskal: 1, prim: 2, eller: 3, wilson: 4,
  growing_tree: 5, binary_tree: 6, sidewinder: 7, aldous_broder: 8, hunt_and_kill: 9,
};

export const SOLVER_ALGO_ID: Record<SolverAlgo, number> = {
  bfs: 0, dfs: 1, astar: 2, dijkstra: 3, greedy_bfs: 4,
  wall_follower: 5, tremaux: 6, dead_end_filling: 7,
};

export interface GraphDataPoint {
  step: number;
  value: number;
}

export interface RunHistoryEntry {
  id: number;
  algoName: string;
  solverAlgo: SolverAlgo;
  generatorAlgo: GeneratorAlgo;
  width: number;
  height: number;
  seed: number;
  steps: number;
  visited: number;
  pathLength: number;
  timeMs: number;
}

export type ToolMode = 'pan' | 'draw' | 'erase' | 'set-start' | 'set-end';

export type PlaybackState = 'idle' | 'generating' | 'gen-paused' | 'solving' | 'solve-paused' | 'done';

export interface Metrics {
  steps_taken: number;
  cells_visited: number;
  path_length: number;
  dead_ends: number;
  frontier_max_size: number;
  elapsed_us: number;
}

export interface CompareResult {
  algo: SolverAlgo;
  algoName: string;
  steps: number;
  visited: number;
  pathLength: number;
  timeMs: number;
  isBest: boolean;
}

/** Snapshot of a single solver run on the same maze. */
export interface ComparisonSolveData {
  algo: SolverAlgo;
  algoName: string;
  wallData: Uint8Array;
  cellStates: Uint8Array;
  solutionPath: number[];
  metrics: Metrics;
}

export interface MazeState {
  width: number;
  setWidth: (w: number) => void;
  height: number;
  setHeight: (h: number) => void;
  seed: number;
  setSeed: (s: number) => void;
  topology: () => Topology;
  setTopology: (t: Topology) => void;
  generatorAlgo: () => GeneratorAlgo;
  setGeneratorAlgo: (a: GeneratorAlgo) => void;
  solverAlgo: () => SolverAlgo;
  setSolverAlgo: (a: SolverAlgo) => void;
  speed: () => number;
  setSpeed: (s: number) => void;
  playbackState: () => PlaybackState;
  setPlaybackState: (s: PlaybackState) => void;
  metrics: () => Metrics;
  setMetrics: (m: Metrics) => void;

  // Maze cell data (wall bitmasks) -- updated from WASM
  wallData: () => Uint8Array | null;
  setWallData: (d: Uint8Array | null) => void;
  solutionPath: () => number[];
  setSolutionPath: (p: number[]) => void;

  // Visited state per cell for rendering
  cellStates: () => Uint8Array | null;
  setCellStates: (d: Uint8Array | null) => void;

  // Comparison mode
  compareMode: () => boolean;
  setCompareMode: (v: boolean) => void;
  compareAlgo: () => SolverAlgo;
  setCompareAlgo: (a: SolverAlgo) => void;
  compareResults: () => CompareResult[];
  setCompareResults: (r: CompareResult[]) => void;

  // Side-by-side comparison data (two completed solver runs)
  comparisonData: () => [ComparisonSolveData, ComparisonSolveData] | null;
  setComparisonData: (d: [ComparisonSolveData, ComparisonSolveData] | null) => void;

  // Tool mode for interactive editing
  toolMode: () => ToolMode;
  setToolMode: (m: ToolMode) => void;

  // Start/end cell indices
  startCell: () => number;
  setStartCell: (c: number) => void;
  endCell: () => number;
  setEndCell: (c: number) => void;

  // Heatmap overlay
  heatmapEnabled: () => boolean;
  setHeatmapEnabled: (v: boolean) => void;

  // Realtime graphs
  graphFrontier: () => GraphDataPoint[];
  setGraphFrontier: (d: GraphDataPoint[]) => void;
  graphVisited: () => GraphDataPoint[];
  setGraphVisited: (d: GraphDataPoint[]) => void;

  // Run history
  runHistory: () => RunHistoryEntry[];
  setRunHistory: (h: RunHistoryEntry[]) => void;
  addRunHistory: (entry: Omit<RunHistoryEntry, 'id'>) => void;

  // Video recording
  isRecording: () => boolean;
  setIsRecording: (v: boolean) => void;
}

export function createMazeStore(): MazeState {
  const [width, setWidth] = createSignal(20);
  const [height, setHeight] = createSignal(20);
  const [seed, setSeed] = createSignal(42);
  const [topology, setTopology] = createSignal<Topology>('rectangular');
  const [generatorAlgo, setGeneratorAlgo] = createSignal<GeneratorAlgo>('dfs');
  const [solverAlgo, setSolverAlgo] = createSignal<SolverAlgo>('bfs');
  const [speed, setSpeed] = createSignal(50);
  const [playbackState, setPlaybackState] = createSignal<PlaybackState>('idle');
  const [metrics, setMetrics] = createSignal<Metrics>({
    steps_taken: 0,
    cells_visited: 0,
    path_length: 0,
    dead_ends: 0,
    frontier_max_size: 0,
    elapsed_us: 0,
  });
  const [wallData, setWallData] = createSignal<Uint8Array | null>(null);
  const [solutionPath, setSolutionPath] = createSignal<number[]>([]);
  const [cellStates, setCellStates] = createSignal<Uint8Array | null>(null);

  const [compareMode, setCompareMode] = createSignal(false);
  const [compareAlgo, setCompareAlgo] = createSignal<SolverAlgo>('astar');
  const [compareResults, setCompareResults] = createSignal<CompareResult[]>([]);
  const [comparisonData, setComparisonData] = createSignal<[ComparisonSolveData, ComparisonSolveData] | null>(null);

  const [toolMode, setToolMode] = createSignal<ToolMode>('pan');
  const [startCell, setStartCell] = createSignal(0);
  const [endCell, setEndCell] = createSignal(width() * height() - 1);

  const [heatmapEnabled, setHeatmapEnabled] = createSignal(false);

  const [graphFrontier, setGraphFrontier] = createSignal<GraphDataPoint[]>([]);
  const [graphVisited, setGraphVisited] = createSignal<GraphDataPoint[]>([]);

  const [runHistory, setRunHistory] = createSignal<RunHistoryEntry[]>([]);
  const [isRecording, setIsRecording] = createSignal(false);
  let nextHistoryId = 1;
  const addRunHistory = (entry: Omit<RunHistoryEntry, 'id'>) => {
    const newEntry: RunHistoryEntry = { ...entry, id: nextHistoryId++ };
    setRunHistory((prev) => [newEntry, ...prev].slice(0, 20));
  };

  return {
    get width() { return width(); },
    setWidth: (w) => setWidth(w),
    get height() { return height(); },
    setHeight: (h) => setHeight(h),
    get seed() { return seed(); },
    setSeed: (s) => setSeed(s),
    topology,
    setTopology,
    generatorAlgo,
    setGeneratorAlgo,
    solverAlgo,
    setSolverAlgo,
    speed,
    setSpeed,
    playbackState,
    setPlaybackState,
    metrics,
    setMetrics,
    wallData,
    setWallData,
    solutionPath,
    setSolutionPath,
    cellStates,
    setCellStates,
    compareMode,
    setCompareMode,
    compareAlgo,
    setCompareAlgo,
    compareResults,
    setCompareResults,
    comparisonData,
    setComparisonData,
    toolMode,
    setToolMode,
    startCell,
    setStartCell,
    endCell,
    setEndCell,
    heatmapEnabled,
    setHeatmapEnabled,
    graphFrontier,
    setGraphFrontier,
    graphVisited,
    setGraphVisited,
    runHistory,
    setRunHistory,
    addRunHistory,
    isRecording,
    setIsRecording,
  };
}
