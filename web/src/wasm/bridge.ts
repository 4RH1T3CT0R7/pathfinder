let wasmModule: any = null;
let engine: any = null;
let wasmMemory: WebAssembly.Memory | null = null;

let cellStateBuffer: Uint8Array | null = null;
let prevWallSnapshot: Uint8Array | null = null;
let visitOrderBuffer: Uint32Array | null = null;
let visitCounter = 0;

// Cell state constants (mirroring the Rust CELL_* constants)
const CELL_UNVISITED = 0;
const CELL_FRONTIER = 1;
const CELL_VISITED = 2;
const CELL_ACTIVE = 3;
const CELL_SOLUTION = 4;
const CELL_BACKTRACKED = 5;

export async function loadWasm(): Promise<void> {
  try {
    wasmModule = await import('../../pkg/pathfinder_core.js');
    await wasmModule.default();
    // Access WASM linear memory via the __wasm internal export
    if (wasmModule.__wasm && wasmModule.__wasm.memory) {
      wasmMemory = wasmModule.__wasm.memory;
    }
  } catch (e) {
    console.warn('WASM not loaded (run wasm-pack build first):', e);
  }
}

export function createEngine(width: number, height: number, seed: number, topology: number = 0): void {
  if (!wasmModule) return;
  const seedHi = (seed >>> 16) & 0xFFFF;
  const seedLo = seed & 0xFFFF;
  engine = new wasmModule.MazeEngine(width, height, seedHi, seedLo, topology);

  // Initialize cell state tracking using actual cell count from engine
  const count = engine.cell_count();
  cellStateBuffer = new Uint8Array(count);
  prevWallSnapshot = new Uint8Array(count).fill(0xFF);
  visitOrderBuffer = new Uint32Array(count);
  visitCounter = 0;
}

export function initGenerator(algo: number, startCell: number): void {
  engine?.init_generator(algo, startCell);

  // Reset cell state tracking for new generation
  if (cellStateBuffer) {
    cellStateBuffer.fill(CELL_UNVISITED);
    visitCounter = 0;
    if (visitOrderBuffer) visitOrderBuffer.fill(0);
  }
  // Snapshot the initial wall state (all walls up)
  if (engine && prevWallSnapshot) {
    const count = prevWallSnapshot.length;
    for (let i = 0; i < count; i++) {
      prevWallSnapshot[i] = engine.cell_walls(i);
    }
  }
}

export function stepGenerate(n: number): number {
  if (!engine) return 0;
  const performed = engine.step_generate(n);

  // Read cell states from Rust engine (which tracks them per-step)
  if (performed > 0 && cellStateBuffer) {
    try {
      const ptr = engine.cell_states_ptr();
      if (ptr && wasmMemory) {
        const count = cellStateBuffer.length;
        const mem = new Uint8Array(wasmMemory.buffer, ptr, count);
        cellStateBuffer.set(mem);
      }
    } catch (_) {
      const count = cellStateBuffer.length;
      for (let i = 0; i < count; i++) {
        cellStateBuffer[i] = engine.cell_state(i);
      }
    }
    if (visitOrderBuffer) {
      try {
        const ptr = engine.visit_order_ptr();
        if (ptr && wasmMemory) {
          const mem = new Uint32Array(wasmMemory.buffer, ptr, visitOrderBuffer.length);
          visitOrderBuffer.set(mem);
        }
      } catch (_) {}
    }
  }

  return performed;
}

export function isGenerationDone(): boolean {
  if (!engine) return true;
  const done = engine.is_generation_done();
  // When generation completes, reset all cell states to unvisited
  // (the maze is built, generation colors are no longer needed)
  if (done && cellStateBuffer) {
    cellStateBuffer.fill(CELL_UNVISITED);
  }
  return done;
}

export function initSolver(algo: number, start: number, end: number): void {
  engine?.init_solver(algo, start, end);

  // Reset cell states for solving phase -- mark everything as unvisited
  if (cellStateBuffer) {
    cellStateBuffer.fill(CELL_UNVISITED);
    visitCounter = 0;
    if (visitOrderBuffer) visitOrderBuffer.fill(0);
  }
}

export function stepSolve(n: number): number {
  if (!engine) return 0;
  const performed = engine.step_solve(n);

  // Read cell states from Rust engine (which tracks them per-step)
  if (performed > 0 && cellStateBuffer) {
    try {
      const ptr = engine.cell_states_ptr();
      if (ptr && wasmMemory) {
        const count = cellStateBuffer.length;
        const mem = new Uint8Array(wasmMemory.buffer, ptr, count);
        cellStateBuffer.set(mem);
      }
    } catch (_) {
      // Fallback: read per-cell
      const count = cellStateBuffer.length;
      for (let i = 0; i < count; i++) {
        cellStateBuffer[i] = engine.cell_state(i);
      }
    }
    // Also update visit order from engine
    if (visitOrderBuffer) {
      try {
        const ptr = engine.visit_order_ptr();
        if (ptr && wasmMemory) {
          const mem = new Uint32Array(wasmMemory.buffer, ptr, visitOrderBuffer.length);
          visitOrderBuffer.set(mem);
        }
      } catch (_) {}
    }
  }

  return performed;
}

export function isSolvingDone(): boolean {
  return engine?.is_solving_done() ?? true;
}

export function getCellWalls(cell: number): number {
  return engine?.cell_walls(cell) ?? 0xF;
}

export function getMetricsJson(): string {
  return engine?.get_metrics_json() ?? '{}';
}

export function getSolutionPathJson(): string {
  return engine?.solution_path_json() ?? '[]';
}

export function getCellCount(): number {
  return engine?.cell_count() ?? 0;
}

export function getTopology(): number {
  return engine?.topology() ?? 0;
}

export function getCellPositions(): Float64Array {
  if (!engine || !wasmModule) return new Float64Array(0);
  try {
    const ptr = engine.cell_positions_ptr();
    const len = engine.cell_positions_len();
    if (ptr && wasmMemory) {
      const mem = new Float64Array(wasmMemory.buffer, ptr, len);
      return new Float64Array(mem);
    }
  } catch (_) {
    // Fall through
  }
  return new Float64Array(0);
}

export function getDirectionsCount(): number {
  return engine?.directions_count() ?? 4;
}

export function getWallData(width: number, height: number): Uint8Array {
  const count = engine ? engine.cell_count() : width * height;
  const data = new Uint8Array(count);
  if (engine) {
    // Try zero-copy via WASM memory if walls_ptr is available
    try {
      const ptr = engine.walls_ptr();
      if (ptr && wasmMemory) {
        const mem = new Uint8Array(wasmMemory.buffer, ptr, count);
        data.set(mem);
        return data;
      }
    } catch (_) {
      // Fall through to per-cell method
    }
    for (let i = 0; i < count; i++) {
      data[i] = engine.cell_walls(i);
    }
  }
  return data;
}

/**
 * Get cell states array for rendering.
 * Reads from JS-side tracking buffer that's updated during stepGenerate/stepSolve.
 */
export function getCellStates(width: number, height: number): Uint8Array {
  const count = engine ? engine.cell_count() : width * height;
  if (cellStateBuffer && cellStateBuffer.length === count) {
    // Return a copy so the signal properly detects changes
    return new Uint8Array(cellStateBuffer);
  }
  return new Uint8Array(count);
}

/**
 * Get visit order array for heatmap rendering.
 */
export function getVisitOrder(width: number, height: number): Uint32Array {
  const count = engine ? engine.cell_count() : width * height;
  if (visitOrderBuffer && visitOrderBuffer.length === count) {
    return new Uint32Array(visitOrderBuffer);
  }
  return new Uint32Array(count);
}

/**
 * Mark solution path cells in the cell state buffer.
 */
export function markSolutionPath(path: number[]): void {
  if (!cellStateBuffer) return;
  for (const cell of path) {
    if (cell >= 0 && cell < cellStateBuffer.length) {
      cellStateBuffer[cell] = CELL_SOLUTION;
    }
  }
}

export function toggleWall(cell: number, direction: number): void {
  engine?.toggle_wall(cell, direction);
}

export function getEngine(): any {
  return engine;
}

// =====================================================
// Multi-engine support for animated comparison mode
// =====================================================

export interface ComparisonLane {
  engine: any;
  algo: string;
  algoId: number;
  done: boolean;
  cellStates: Uint8Array;
  wallData: Uint8Array;
  solutionPath: number[];
  metrics: { steps_taken: number; cells_visited: number; path_length: number };
  cellPositions?: Float64Array;
}

let comparisonLanes: ComparisonLane[] = [];

/**
 * Create N comparison lanes, each with its own WASM engine instance.
 * All share the same maze (same seed, size, generator).
 */
export function createComparisonLanes(
  width: number, height: number, seed: number, topology: number,
  genAlgoId: number, solverAlgoIds: { algo: string; id: number }[],
  startCell: number, endCell: number,
): void {
  if (!wasmModule) return;
  comparisonLanes = [];
  const seedHi = (seed >>> 16) & 0xFFFF;
  const seedLo = seed & 0xFFFF;

  for (const { algo, id } of solverAlgoIds) {
    // Create independent engine
    const eng = new wasmModule.MazeEngine(width, height, seedHi, seedLo, topology);
    // Generate maze instantly
    eng.init_generator(genAlgoId, 0);
    let guard = 0;
    while (!eng.is_generation_done() && guard < 100) {
      eng.step_generate(999999);
      guard++;
    }
    // Init solver
    eng.init_solver(id, startCell, endCell);

    const count = eng.cell_count();
    comparisonLanes.push({
      engine: eng,
      algo,
      algoId: id,
      done: false,
      cellStates: new Uint8Array(count),
      wallData: new Uint8Array(count),
      solutionPath: [],
      metrics: { steps_taken: 0, cells_visited: 0, path_length: 0 },
    });

    // Read initial wall data
    const lane = comparisonLanes[comparisonLanes.length - 1];
    for (let i = 0; i < count; i++) {
      lane.wallData[i] = eng.cell_walls(i);
    }

    // Read cell positions for non-rectangular topologies
    try {
      const ptr = eng.cell_positions_ptr();
      const len = eng.cell_positions_len();
      if (ptr && wasmMemory && len > 0) {
        const mem = new Float64Array(wasmMemory.buffer, ptr, len);
        lane.cellPositions = new Float64Array(mem);
      }
    } catch (_) {
      // Not all topologies provide positions
    }
  }
}

/**
 * Step all comparison lanes by N steps. Returns true if all are done.
 */
export function stepComparisonLanes(n: number): boolean {
  let allDone = true;
  for (const lane of comparisonLanes) {
    if (lane.done) continue;
    lane.engine.step_solve(n);

    // Read cell states from engine
    const count = lane.cellStates.length;
    try {
      const ptr = lane.engine.cell_states_ptr();
      if (ptr && wasmMemory) {
        const mem = new Uint8Array(wasmMemory.buffer, ptr, count);
        lane.cellStates.set(mem);
      }
    } catch (_) {
      for (let i = 0; i < count; i++) {
        lane.cellStates[i] = lane.engine.cell_state(i);
      }
    }

    // Read metrics
    try {
      const m = JSON.parse(lane.engine.get_metrics_json());
      lane.metrics = { steps_taken: m.steps_taken, cells_visited: m.cells_visited, path_length: m.path_length };
    } catch (_) {}

    if (lane.engine.is_solving_done()) {
      lane.done = true;
      try {
        lane.solutionPath = JSON.parse(lane.engine.solution_path_json());
        // Mark solution in cell states
        for (const cell of lane.solutionPath) {
          if (cell >= 0 && cell < count) lane.cellStates[cell] = CELL_SOLUTION;
        }
      } catch (_) {}
    } else {
      allDone = false;
    }
  }
  return allDone;
}

/**
 * Get current lane snapshots for rendering.
 */
export function getComparisonLaneData(): ComparisonLane[] {
  return comparisonLanes.map(lane => ({
    ...lane,
    cellStates: new Uint8Array(lane.cellStates),
    wallData: new Uint8Array(lane.wallData),
    solutionPath: [...lane.solutionPath],
    cellPositions: lane.cellPositions ? new Float64Array(lane.cellPositions) : undefined,
  }));
}

export function clearComparisonLanes(): void {
  comparisonLanes = [];
}
