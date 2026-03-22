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
      if (ptr && wasmModule && wasmModule.memory) {
        const count = cellStateBuffer.length;
        const mem = new Uint8Array(wasmModule.memory.buffer, ptr, count);
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
        if (ptr && wasmModule && wasmModule.memory) {
          const mem = new Uint32Array(wasmModule.memory.buffer, ptr, visitOrderBuffer.length);
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
  // When generation completes, mark all cells as visited
  if (done && cellStateBuffer) {
    const count = cellStateBuffer.length;
    for (let i = 0; i < count; i++) {
      if (cellStateBuffer[i] !== CELL_UNVISITED) {
        cellStateBuffer[i] = CELL_VISITED;
      }
    }
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
      if (ptr && wasmModule && wasmModule.memory) {
        const count = cellStateBuffer.length;
        const mem = new Uint8Array(wasmModule.memory.buffer, ptr, count);
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
        if (ptr && wasmModule && wasmModule.memory) {
          const mem = new Uint32Array(wasmModule.memory.buffer, ptr, visitOrderBuffer.length);
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
    if (ptr && wasmModule.memory) {
      const mem = new Float64Array(wasmModule.memory.buffer, ptr, len);
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
      if (ptr && wasmModule && wasmModule.memory) {
        const mem = new Uint8Array(wasmModule.memory.buffer, ptr, count);
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
