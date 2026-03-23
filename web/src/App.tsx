import { Component, createSignal, onMount, Show } from 'solid-js';
import Header from './components/layout/Header';
import Sidebar from './components/layout/Sidebar';
import MazeCanvas from './components/maze/MazeCanvas';
import ComparisonView from './components/maze/ComparisonView';
import CompareResults from './components/metrics/CompareResults';
import PlaybackControls from './components/controls/PlaybackControls';
import MetricsPanel from './components/metrics/MetricsPanel';
import RealtimeGraph from './components/metrics/RealtimeGraph';
import RunHistory from './components/metrics/RunHistory';
import {
  createMazeStore,
  GENERATOR_ALGO_ID,
  SOLVER_ALGO_ID,
  ALL_SOLVER_ALGOS,
  SOLVER_ALGO_NAMES,
  TOPOLOGY_ID,
  RECT_ONLY_SOLVERS,
} from './stores/maze';
import type { SolverAlgo, Metrics, ComparisonSolveData, RunHistoryEntry } from './stores/maze';
import * as wasm from './wasm/bridge';
import { parseHash, applyHashToStore } from './hooks/useShareUrl';

const App: Component = () => {
  const store = createMazeStore();
  const [showSidebar, setShowSidebar] = createSignal(false);
  const [showCompareResults, setShowCompareResults] = createSignal(false);
  let animFrameId: number | null = null;
  let timeoutId: number | null = null;
  let hashAutoGenerate = false;
  let solveStartTime = 0;
  let graphStepCounter = 0;
  let mazeCanvasEl: HTMLCanvasElement | undefined;

  // Snapshot history for step-back (circular buffer, max 500 snapshots)
  interface Snapshot {
    wallData: Uint8Array;
    cellStates: Uint8Array;
    metricsJson: string;
  }
  const snapshots: Snapshot[] = [];
  const MAX_SNAPSHOTS = 500;

  const pushSnapshot = () => {
    const wallData = wasm.getWallData(store.activeWidth(), store.activeHeight());
    const cellStates = wasm.getCellStates(store.activeWidth(), store.activeHeight());
    const metricsJson = wasm.getMetricsJson();
    if (snapshots.length >= MAX_SNAPSHOTS) snapshots.shift();
    snapshots.push({ wallData, cellStates, metricsJson });
  };

  const popSnapshot = (): Snapshot | undefined => {
    return snapshots.pop();
  };

  // Speed 0% = 1 step + 200ms delay (very slow)
  // Speed 50% = 10 steps + 16ms delay (visible)
  // Speed 80% = 1000 steps per frame (fast)
  // Speed 100% = all remaining (instant)
  function speedToBatchSize(speed: number): number {
    if (speed >= 100) return 999999;
    if (speed >= 80) return Math.floor(Math.pow(10, (speed - 80) / 6.67));  // 1-1000
    return Math.max(1, Math.floor(Math.pow(10, speed / 40)));  // 1-100
  }

  function speedToDelay(speed: number): number {
    if (speed >= 80) return 0;   // no delay, rAF only
    if (speed >= 50) return 0;   // rAF pacing is enough
    // 0% = 200ms, 50% = 0ms
    return Math.floor(200 * (1 - speed / 50));
  }

  const stopAnimation = () => {
    if (animFrameId !== null) {
      cancelAnimationFrame(animFrameId);
      animFrameId = null;
    }
    if (timeoutId !== null) {
      clearTimeout(timeoutId);
      timeoutId = null;
    }
  };

  const updateRenderState = () => {
    const wallData = wasm.getWallData(store.activeWidth(), store.activeHeight());
    store.setWallData(wallData);
    const states = wasm.getCellStates(store.activeWidth(), store.activeHeight());
    store.setCellStates(states);
    const metricsJson = wasm.getMetricsJson();
    try { store.setMetrics(JSON.parse(metricsJson)); } catch {}
  };

  const scheduleNext = (callback: () => void) => {
    const delay = speedToDelay(store.speed());
    if (delay > 0) {
      timeoutId = window.setTimeout(() => {
        animFrameId = requestAnimationFrame(callback);
      }, delay);
    } else {
      animFrameId = requestAnimationFrame(callback);
    }
  };

  const animateGeneration = () => {
    const state = store.playbackState();
    if (state === 'gen-paused') return;

    const batch = speedToBatchSize(store.speed());
    const performed = wasm.stepGenerate(batch);

    if (performed > 0) updateRenderState();

    if (wasm.isGenerationDone()) {
      updateRenderState();
      store.setPlaybackState('idle');
      stopAnimation();
      return;
    }

    scheduleNext(animateGeneration);
  };

  const animateSolving = () => {
    const state = store.playbackState();
    if (state === 'solve-paused') return;

    const batch = speedToBatchSize(store.speed());
    const performed = wasm.stepSolve(batch);

    if (performed > 0) {
      updateRenderState();

      // Collect graph data points (sample every N steps)
      graphStepCounter += performed;
      const totalCells = wasm.getCellCount() || store.width * store.height;
      const sampleInterval = Math.max(1, Math.floor(totalCells / 200));
      if (graphStepCounter % sampleInterval < performed || performed >= sampleInterval) {
        const states = store.cellStates();
        if (states) {
          let frontierCount = 0;
          let visitedCount = 0;
          for (let i = 0; i < states.length; i++) {
            if (states[i] === 1) frontierCount++;
            if (states[i] >= 1) visitedCount++;
          }
          store.setGraphFrontier([...store.graphFrontier(), { step: graphStepCounter, value: frontierCount }]);
          store.setGraphVisited([...store.graphVisited(), { step: graphStepCounter, value: visitedCount }]);
        }
      }
    }

    if (wasm.isSolvingDone()) {
      const pathJson = wasm.getSolutionPathJson();
      try {
        const path = JSON.parse(pathJson);
        store.setSolutionPath(path);
        wasm.markSolutionPath(path);
      } catch {}
      updateRenderState();

      // Record run history
      const solveEndTime = performance.now();
      const m = store.metrics();
      store.addRunHistory({
        algoName: SOLVER_ALGO_NAMES[store.solverAlgo()],
        solverAlgo: store.solverAlgo(),
        generatorAlgo: store.generatorAlgo(),
        width: store.width,
        height: store.height,
        seed: store.seed,
        steps: m.steps_taken,
        visited: m.cells_visited,
        pathLength: m.path_length,
        timeMs: solveEndTime - solveStartTime,
      });

      store.setPlaybackState('done');
      stopAnimation();
      return;
    }

    scheduleNext(animateSolving);
  };

  const handleGenerate = async () => {
    stopAnimation();
    store.setComparisonData(null);
    store.setCompareResults([]);
    store.setGraphFrontier([]);
    store.setGraphVisited([]);
    setShowCompareResults(false);
    // Reset start/end to defaults for new maze dimensions
    store.setStartCell(0);
    const topologyId = TOPOLOGY_ID[store.topology()];
    await wasm.loadWasm();
    // Lock in the active dimensions at generation time
    store.setActiveWidth(store.width);
    store.setActiveHeight(store.height);
    wasm.createEngine(store.width, store.height, store.seed, topologyId);
    const cellCount = wasm.getCellCount();
    store.setEndCell(cellCount > 0 ? cellCount - 1 : 0);
    const genAlgoId = GENERATOR_ALGO_ID[store.generatorAlgo()];
    wasm.initGenerator(genAlgoId, 0);
    store.setSolutionPath([]);
    store.setCellStates(null);
    snapshots.length = 0;
    store.setPlaybackState('generating');
    setShowSidebar(false);
    animateGeneration();
  };

  /**
   * Run a single solver to completion (instant, no animation) on the current
   * WASM engine. Returns the solve data snapshot. The engine must already have
   * a generated maze. After this call the engine's solver state is consumed.
   */
  const runSolverInstant = (algo: SolverAlgo): ComparisonSolveData => {
    const w = store.width;
    const h = store.height;
    const solveAlgoId = SOLVER_ALGO_ID[algo];

    wasm.initSolver(solveAlgoId, store.startCell(), store.endCell());

    // Run all steps at once
    const MAX_STEPS = 999999;
    wasm.stepSolve(MAX_STEPS);
    while (!wasm.isSolvingDone()) {
      wasm.stepSolve(MAX_STEPS);
    }

    // Collect results
    let solutionPath: number[] = [];
    try {
      solutionPath = JSON.parse(wasm.getSolutionPathJson());
    } catch {}
    wasm.markSolutionPath(solutionPath);

    const wallData = wasm.getWallData(w, h);
    const cellStates = wasm.getCellStates(w, h);
    let metrics: Metrics = {
      steps_taken: 0, cells_visited: 0, path_length: 0,
      dead_ends: 0, frontier_max_size: 0, elapsed_us: 0,
    };
    try { metrics = JSON.parse(wasm.getMetricsJson()); } catch {}

    return {
      algo,
      algoName: SOLVER_ALGO_NAMES[algo],
      wallData,
      cellStates,
      solutionPath,
      metrics,
    };
  };

  /**
   * Re-create the engine with the same seed/size/generator and run generation
   * to completion instantly. This ensures we have a fresh maze identical to
   * what was generated before.
   */
  const regenerateMazeInstant = async () => {
    await wasm.loadWasm();
    const topologyId = TOPOLOGY_ID[store.topology()];
    wasm.createEngine(store.width, store.height, store.seed, topologyId);
    const genAlgoId = GENERATOR_ALGO_ID[store.generatorAlgo()];
    wasm.initGenerator(genAlgoId, 0);
    const MAX_STEPS = 999999;
    wasm.stepGenerate(MAX_STEPS);
    while (!wasm.isGenerationDone()) {
      wasm.stepGenerate(MAX_STEPS);
    }
  };

  const animateComparison = () => {
    const state = store.playbackState();
    if (state === 'solve-paused') return;

    const batch = speedToBatchSize(store.speed());
    const allDone = wasm.stepComparisonLanes(batch);

    // Update comparison data from lanes
    const lanes = wasm.getComparisonLaneData();
    const data: ComparisonSolveData[] = lanes.map(lane => ({
      algo: lane.algo as SolverAlgo,
      algoName: SOLVER_ALGO_NAMES[lane.algo as SolverAlgo] || lane.algo,
      wallData: lane.wallData,
      cellStates: lane.cellStates,
      solutionPath: lane.solutionPath,
      metrics: {
        steps_taken: lane.metrics.steps_taken,
        cells_visited: lane.metrics.cells_visited,
        path_length: lane.metrics.path_length,
        dead_ends: 0,
        frontier_max_size: 0,
        elapsed_us: 0,
      },
      cellPositions: lane.cellPositions,
    }));
    store.setComparisonData(data);

    if (allDone) {
      store.setPlaybackState('done');
      stopAnimation();
      return;
    }

    scheduleNext(animateComparison);
  };

  const handleCompareSolve = async () => {
    if (!store.wallData()) return;
    stopAnimation();

    const algos = store.compareAlgos();
    if (algos.length < 2) return;

    const topologyId = TOPOLOGY_ID[store.topology()];
    const genAlgoId = GENERATOR_ALGO_ID[store.generatorAlgo()];
    await wasm.loadWasm();

    // Create independent engines for each algorithm
    wasm.createComparisonLanes(
      store.activeWidth(), store.activeHeight(), store.seed, topologyId,
      genAlgoId,
      algos.map(a => ({ algo: a, id: SOLVER_ALGO_ID[a] })),
      store.startCell(),
      store.endCell(),
    );

    store.setPlaybackState('solving');
    setShowSidebar(false);
    animateComparison();
  };

  const handleAutoCompare = async () => {
    if (!store.wallData()) return;
    stopAnimation();

    const results: { algo: SolverAlgo; algoName: string; steps: number; visited: number; pathLength: number; timeMs: number; isBest: boolean }[] = [];

    const applicableAlgos = ALL_SOLVER_ALGOS.filter(
      a => store.topology() === 'rectangular' || !RECT_ONLY_SOLVERS.includes(a)
    );
    for (const algo of applicableAlgos) {
      await regenerateMazeInstant();

      const t0 = performance.now();
      const data = runSolverInstant(algo);
      const elapsed = performance.now() - t0;

      results.push({
        algo,
        algoName: SOLVER_ALGO_NAMES[algo],
        steps: data.metrics.steps_taken,
        visited: data.metrics.cells_visited,
        pathLength: data.metrics.path_length,
        timeMs: elapsed,
        isBest: false,
      });
    }

    // Determine "best": shortest path (non-zero), then fewest steps
    // If multiple are tied, no single winner
    const withPath = results.filter((r) => r.pathLength > 0);
    if (withPath.length > 0) {
      withPath.sort((a, b) => {
        if (a.pathLength !== b.pathLength) return a.pathLength - b.pathLength;
        return a.steps - b.steps;
      });
      const best = withPath[0];
      // Count how many share the exact same best score
      const tiedCount = withPath.filter(
        r => r.pathLength === best.pathLength && r.steps === best.steps
      ).length;
      // Only mark winner if there's a unique best
      if (tiedCount === 1) {
        for (const r of results) {
          r.isBest = r.algo === best.algo;
        }
      }
      // Otherwise all isBest stay false — tie
    }

    store.setCompareResults(results);
    setShowCompareResults(true);
    store.setPlaybackState('done');
    setShowSidebar(false);
  };

  const handleSolve = () => {
    if (!store.wallData()) return;
    stopAnimation();

    if (store.compareMode()) {
      void handleCompareSolve();
      return;
    }

    store.setComparisonData(null);
    store.setGraphFrontier([]);
    store.setGraphVisited([]);
    graphStepCounter = 0;
    solveStartTime = performance.now();
    const solveAlgoId = SOLVER_ALGO_ID[store.solverAlgo()];
    wasm.initSolver(solveAlgoId, store.startCell(), store.endCell());
    store.setSolutionPath([]);
    store.setCellStates(null);
    store.setPlaybackState('solving');
    setShowSidebar(false);
    animateSolving();
  };

  const handlePause = () => {
    const state = store.playbackState();
    if (state === 'generating') {
      stopAnimation();
      store.setPlaybackState('gen-paused');
    } else if (state === 'solving') {
      stopAnimation();
      store.setPlaybackState('solve-paused');
    } else if (state === 'gen-paused') {
      store.setPlaybackState('generating');
      animateGeneration();
    } else if (state === 'solve-paused') {
      store.setPlaybackState('solving');
      animateSolving();
    }
  };

  const handleStep = () => {
    const state = store.playbackState();
    if (state === 'gen-paused' || state === 'generating') {
      stopAnimation();
      store.setPlaybackState('gen-paused');
      pushSnapshot(); // save state before stepping
      wasm.stepGenerate(1);
      updateRenderState();
      if (wasm.isGenerationDone()) store.setPlaybackState('idle');
    } else if (state === 'solve-paused' || state === 'solving') {
      stopAnimation();
      store.setPlaybackState('solve-paused');
      pushSnapshot(); // save state before stepping
      wasm.stepSolve(1);
      updateRenderState();
      if (wasm.isSolvingDone()) {
        const pathJson = wasm.getSolutionPathJson();
        try { const path = JSON.parse(pathJson); store.setSolutionPath(path); wasm.markSolutionPath(path); } catch {}
        updateRenderState();
        store.setPlaybackState('done');
      }
    }
  };

  const handleStepBack = () => {
    const state = store.playbackState();
    if (state !== 'gen-paused' && state !== 'solve-paused') return;
    const snapshot = popSnapshot();
    if (!snapshot) return;
    // Restore visual state from snapshot
    store.setWallData(snapshot.wallData);
    store.setCellStates(snapshot.cellStates);
    try { store.setMetrics(JSON.parse(snapshot.metricsJson)); } catch {}
  };

  const handleReset = () => {
    stopAnimation();
    store.setPlaybackState('idle');
    store.setWallData(null);
    store.setSolutionPath([]);
    store.setCellStates(null);
    store.setComparisonData(null);
    store.setCompareResults([]);
    store.setGraphFrontier([]);
    store.setGraphVisited([]);
    setShowCompareResults(false);
    store.setStartCell(0);
    const cellCount = wasm.getCellCount();
    store.setEndCell(cellCount > 0 ? cellCount - 1 : store.width * store.height - 1);
    store.setToolMode('pan');
    store.setMetrics({
      steps_taken: 0, cells_visited: 0, path_length: 0,
      dead_ends: 0, frontier_max_size: 0, elapsed_us: 0,
    });
  };

  const handleReRun = async (entry: RunHistoryEntry) => {
    store.setWidth(entry.width);
    store.setHeight(entry.height);
    store.setSeed(entry.seed);
    store.setGeneratorAlgo(entry.generatorAlgo);
    store.setSolverAlgo(entry.solverAlgo);
    store.setStartCell(0);
    await regenerateMazeInstant();
    const cellCount = wasm.getCellCount();
    store.setEndCell(cellCount > 0 ? cellCount - 1 : entry.width * entry.height - 1);
    updateRenderState();
    store.setPlaybackState('idle');
    handleSolve();
  };

  const handleRecordVideo = async () => {
    if (!mazeCanvasEl || !store.wallData()) return;
    if (store.isRecording()) return;

    stopAnimation();
    store.setIsRecording(true);

    // Regenerate maze from scratch for a clean recording
    await regenerateMazeInstant();
    updateRenderState();

    const canvas = mazeCanvasEl;
    const stream = canvas.captureStream(30);
    const chunks: Blob[] = [];
    const recorder = new MediaRecorder(stream, { mimeType: 'video/webm' });

    recorder.ondataavailable = (e) => {
      if (e.data.size > 0) chunks.push(e.data);
    };

    recorder.onstop = () => {
      const blob = new Blob(chunks, { type: 'video/webm' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `maze_${store.width}x${store.height}_s${store.seed}.webm`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      store.setIsRecording(false);
    };

    recorder.start();

    // Init solver and animate at 75% speed
    const solveAlgoId = SOLVER_ALGO_ID[store.solverAlgo()];
    wasm.initSolver(solveAlgoId, store.startCell(), store.endCell());
    store.setSolutionPath([]);
    store.setCellStates(null);

    const recordSpeed = 75;
    const recordBatch = speedToBatchSize(recordSpeed);

    const animateRecord = () => {
      const performed = wasm.stepSolve(recordBatch);
      if (performed > 0) updateRenderState();

      if (wasm.isSolvingDone()) {
        const pathJson = wasm.getSolutionPathJson();
        try {
          const path = JSON.parse(pathJson);
          store.setSolutionPath(path);
          wasm.markSolutionPath(path);
        } catch {}
        updateRenderState();
        store.setPlaybackState('done');

        // Hold the final frame for 500ms before stopping
        setTimeout(() => {
          recorder.stop();
        }, 500);
        return;
      }

      requestAnimationFrame(animateRecord);
    };

    store.setPlaybackState('solving');
    requestAnimationFrame(animateRecord);
  };

  // On mount: if URL has hash params, apply them and auto-generate
  onMount(() => {
    const params = parseHash(window.location.hash);
    if (params) {
      applyHashToStore(params, store);
      hashAutoGenerate = true;
      // Defer generation to next tick so the store is fully initialized
      queueMicrotask(() => {
        if (hashAutoGenerate) {
          hashAutoGenerate = false;
          void handleGenerate();
        }
      });
    }
  });

  return (
    <div style={{
      display: 'flex',
      'flex-direction': 'column',
      height: '100%',
      overflow: 'hidden',
    }}>
      <Header
        store={store}
        onToggleSidebar={() => setShowSidebar((v) => !v)}
      />
      <div class="app-body" style={{
        display: 'flex',
        flex: '1',
        overflow: 'hidden',
        position: 'relative',
      }}>
        {/* Mobile overlay backdrop */}
        <div
          class="sidebar-overlay"
          classList={{ 'sidebar-overlay--visible': showSidebar() }}
          onClick={() => setShowSidebar(false)}
        />
        <div classList={{ 'sidebar-open': showSidebar() }} style={{ height: '100%', overflow: 'hidden' }}>
          <Sidebar
            store={store}
            onGenerate={handleGenerate}
            onSolve={handleSolve}
            onReset={handleReset}
            onAutoCompare={handleAutoCompare}
            onRecordVideo={handleRecordVideo}
          />
        </div>
        <main style={{
          flex: '1',
          display: 'flex',
          'flex-direction': 'column',
          overflow: 'hidden',
          'min-width': '0',
        }}>
          <div style={{
            flex: '1',
            position: 'relative',
            overflow: 'hidden',
            background: 'var(--bg)',
            display: 'flex',
            'align-items': 'center',
            'justify-content': 'center',
          }}>
            <Show
              when={store.comparisonData()}
              fallback={
                <MazeCanvas
                  store={store}
                  canvasRef={(el) => { mazeCanvasEl = el; }}
                />
              }
            >
              {(data) => (
                <ComparisonView
                  data={data()}
                  width={store.activeWidth()}
                  height={store.activeHeight()}
                  topology={store.topology()}
                />
              )}
            </Show>

            {/* Auto Compare Results overlay */}
            <Show when={showCompareResults() && store.compareResults().length > 0}>
              <CompareResults
                results={store.compareResults()}
                onClose={() => setShowCompareResults(false)}
              />
            </Show>
          </div>
          <PlaybackControls
            store={store}
            onPause={handlePause}
            onStep={handleStep}
            onStepBack={handleStepBack}
            onReset={handleReset}
          />
          <MetricsPanel store={store} />
          <Show when={store.playbackState() === 'solving' || store.playbackState() === 'done' || store.playbackState() === 'solve-paused'}>
            <div style={{ display: 'flex', gap: '4px', padding: '4px 20px', background: 'var(--bg2)', 'border-top': '1px solid var(--border)', 'flex-shrink': '0' }}>
              <RealtimeGraph data={store.graphFrontier} color="var(--blue)" label="Frontier" />
              <RealtimeGraph data={store.graphVisited} color="var(--purple)" label="Visited" />
            </div>
          </Show>
          <RunHistory history={store.runHistory} onReRun={handleReRun} />
        </main>
      </div>
    </div>
  );
};

export default App;