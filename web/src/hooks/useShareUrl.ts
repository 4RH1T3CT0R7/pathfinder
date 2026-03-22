import type { MazeState, GeneratorAlgo, SolverAlgo } from '../stores/maze';

const GENERATOR_ALGOS: readonly GeneratorAlgo[] = [
  'dfs', 'kruskal', 'prim', 'eller', 'wilson',
  'growing_tree', 'binary_tree', 'sidewinder', 'aldous_broder', 'hunt_and_kill',
];
const SOLVER_ALGOS: readonly SolverAlgo[] = [
  'bfs', 'dfs', 'astar', 'dijkstra', 'greedy_bfs',
  'wall_follower', 'tremaux', 'dead_end_filling',
];

export interface HashParams {
  seed: number;
  width: number;
  height: number;
  generator: GeneratorAlgo;
  solver: SolverAlgo;
}

/** Build a URL hash string from the current store state. */
export function buildHash(store: MazeState): string {
  const parts = [
    `s=${store.seed}`,
    `w=${store.width}`,
    `h=${store.height}`,
    `g=${store.generatorAlgo()}`,
    `v=${store.solverAlgo()}`,
  ];
  return '#' + parts.join('&');
}

/** Parse URL hash into params. Returns null if hash is empty or invalid. */
export function parseHash(hash: string): HashParams | null {
  if (!hash || hash.length < 2) return null;

  const raw = hash.startsWith('#') ? hash.slice(1) : hash;
  const map = new Map<string, string>();
  for (const segment of raw.split('&')) {
    const eq = segment.indexOf('=');
    if (eq > 0) {
      map.set(segment.slice(0, eq), segment.slice(eq + 1));
    }
  }

  const sStr = map.get('s');
  const wStr = map.get('w');
  const hStr = map.get('h');
  const gStr = map.get('g');
  const vStr = map.get('v');

  if (!sStr || !wStr || !hStr) return null;

  const seed = parseInt(sStr, 10);
  const width = parseInt(wStr, 10);
  const height = parseInt(hStr, 10);

  if (isNaN(seed) || isNaN(width) || isNaN(height)) return null;
  if (width < 3 || width > 1000 || height < 3 || height > 1000) return null;

  const generator: GeneratorAlgo = (gStr && GENERATOR_ALGOS.includes(gStr as GeneratorAlgo))
    ? gStr as GeneratorAlgo
    : 'dfs';

  const solver: SolverAlgo = (vStr && SOLVER_ALGOS.includes(vStr as SolverAlgo))
    ? vStr as SolverAlgo
    : 'bfs';

  return { seed, width, height, generator, solver };
}

/** Apply parsed hash params to the store. */
export function applyHashToStore(params: HashParams, store: MazeState): void {
  store.setSeed(params.seed);
  store.setWidth(params.width);
  store.setHeight(params.height);
  store.setGeneratorAlgo(params.generator);
  store.setSolverAlgo(params.solver);
}

/** Update the browser URL hash without triggering navigation. */
export function updateUrlHash(store: MazeState): void {
  const hash = buildHash(store);
  window.history.replaceState(null, '', hash);
}

/** Copy the current share URL to the clipboard. Returns a promise. */
export async function copyShareUrl(store: MazeState): Promise<void> {
  updateUrlHash(store);
  const url = window.location.href;
  await navigator.clipboard.writeText(url);
}
