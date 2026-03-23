import { Component, createEffect, createSignal, onMount, onCleanup } from 'solid-js';
import type { ComparisonSolveData } from '../../stores/maze';
import { renderMazeToCanvas } from '../../rendering/mazeRenderer';
import type { MazeRenderData } from '../../rendering/mazeRenderer';

function formatNumber(n: number): string {
  if (n >= 1000000) return (n / 1000000).toFixed(1) + 'M';
  if (n >= 1000) return n.toLocaleString();
  return String(n);
}

interface PaneProps {
  data: ComparisonSolveData;
  width: number;
  height: number;
  isWinner: boolean;
  topology?: string;
}

const ComparisonPane: Component<PaneProps> = (props) => {
  let canvasRef: HTMLCanvasElement | undefined;
  let containerRef: HTMLDivElement | undefined;

  // Per-pane zoom/pan state
  const [viewScale, setViewScale] = createSignal(1);
  const [viewOffsetX, setViewOffsetX] = createSignal(0);
  const [viewOffsetY, setViewOffsetY] = createSignal(0);
  let isPanning = false;
  let panStartX = 0;
  let panStartY = 0;
  let panStartOX = 0;
  let panStartOY = 0;

  const renderMaze = () => {
    const canvas = canvasRef;
    const container = containerRef;
    if (!canvas || !container) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const displayWidth = container.clientWidth;
    const displayHeight = container.clientHeight;
    if (displayWidth === 0 || displayHeight === 0) return;

    canvas.width = displayWidth * dpr;
    canvas.height = displayHeight * dpr;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    ctx.fillStyle = '#06080d';
    ctx.fillRect(0, 0, displayWidth, displayHeight);

    const wallData = props.data.wallData;
    const solutionPath = props.data.solutionPath;
    const cellStates = props.data.cellStates;
    const w = props.width;
    const h = props.height;
    const topology = props.topology || 'rectangular';

    if (!wallData || wallData.length === 0) return;

    // Determine start/end cells
    const totalCells = wallData.length;
    const startCell = 0;
    const endCell = totalCells - 1;

    // Build render data
    const renderData: MazeRenderData = {
      wallData,
      cellStates,
      solutionPath,
      w,
      h,
      startCell,
      endCell,
      topology,
      cellPositions: props.data.cellPositions,
    };

    // Apply zoom/pan
    const scale = viewScale();
    const ox = viewOffsetX();
    const oy = viewOffsetY();

    ctx.save();

    if (topology === 'rectangular') {
      // For rect: zoom/scale centered on maze
      const padding = 4;
      const availW = displayWidth - 2 * padding;
      const availH = displayHeight - 2 * padding;
      const cellSize = Math.max(1, Math.min(availW / w, availH / h));
      const mazeW = cellSize * w;
      const mazeH = cellSize * h;

      ctx.translate(
        ox + (displayWidth / 2) * (1 - scale),
        oy + (displayHeight / 2) * (1 - scale),
      );
      ctx.scale(scale, scale);
    } else {
      // For non-rect: zoom around center
      const cx = displayWidth / 2;
      const cy = displayHeight / 2;
      ctx.translate(cx + ox, cy + oy);
      ctx.scale(scale, scale);
      ctx.translate(-cx, -cy);
    }

    renderMazeToCanvas(ctx, renderData, displayWidth, displayHeight);

    ctx.restore();
  };

  const handleWheel = (e: WheelEvent) => {
    e.preventDefault();
    const factor = e.deltaY < 0 ? 1.15 : 1 / 1.15;
    setViewScale(s => Math.max(0.3, Math.min(8, s * factor)));
  };

  const handlePointerDown = (e: PointerEvent) => {
    if (e.button !== 0) return;
    isPanning = true;
    panStartX = e.clientX;
    panStartY = e.clientY;
    panStartOX = viewOffsetX();
    panStartOY = viewOffsetY();
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  };

  const handlePointerMove = (e: PointerEvent) => {
    if (!isPanning) return;
    setViewOffsetX(panStartOX + e.clientX - panStartX);
    setViewOffsetY(panStartOY + e.clientY - panStartY);
  };

  const handlePointerUp = (e: PointerEvent) => {
    isPanning = false;
    (e.target as HTMLElement).releasePointerCapture(e.pointerId);
  };

  const resetView = () => {
    setViewScale(1);
    setViewOffsetX(0);
    setViewOffsetY(0);
  };

  onMount(() => {
    const observer = new ResizeObserver(() => renderMaze());
    if (containerRef) observer.observe(containerRef);
    renderMaze();
    onCleanup(() => observer.disconnect());
  });

  createEffect(() => {
    const d = props.data;
    void d.wallData;
    void d.cellStates;
    void d.solutionPath;
    void d.metrics;
    void d.cellPositions;
    void props.width;
    void props.height;
    void props.isWinner;
    void props.topology;
    viewScale();
    viewOffsetX();
    viewOffsetY();
    queueMicrotask(() => renderMaze());
  });

  return (
    <div style={{
      width: '100%',
      height: '100%',
      display: 'flex',
      'flex-direction': 'column',
      'min-width': '0',
      overflow: 'hidden',
    }}>
      {/* Algorithm label */}
      <div style={{
        height: '36px',
        display: 'flex',
        'align-items': 'center',
        'justify-content': 'center',
        gap: '8px',
        background: 'var(--bg2)',
        'border-bottom': '1px solid var(--border)',
        'font-family': 'var(--font-mono)',
        'font-size': '12px',
        'font-weight': 600,
        color: 'var(--text)',
        'flex-shrink': '0',
      }}>
        <span>{props.data.algoName}</span>
        {props.isWinner && (
          <span style={{
            background: 'rgba(0,212,170,0.15)',
            color: 'var(--cyan)',
            'font-size': '9px',
            'text-transform': 'uppercase',
            'letter-spacing': '0.1em',
            padding: '2px 8px',
            'border-radius': '4px',
            border: '1px solid var(--cyan)',
          }}>
            winner
          </span>
        )}
      </div>

      {/* Canvas with zoom/pan */}
      <div
        ref={containerRef}
        onWheel={handleWheel}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onDblClick={resetView}
        style={{
          flex: '1',
          position: 'relative',
          background: 'var(--bg)',
          overflow: 'hidden',
          cursor: 'grab',
          'touch-action': 'none',
        }}
      >
        <canvas
          ref={canvasRef}
          style={{
            display: 'block',
            width: '100%',
            height: '100%',
            position: 'absolute',
            top: '0',
            left: '0',
          }}
        />
      </div>

      {/* Mini metrics bar */}
      <div style={{
        height: '44px',
        background: 'var(--bg2)',
        'border-top': '1px solid var(--border)',
        display: 'flex',
        'align-items': 'center',
        'justify-content': 'center',
        gap: '16px',
        'flex-shrink': '0',
        'font-family': 'var(--font-mono)',
        'font-size': '11px',
      }}>
        <div>
          <span style={{ color: 'var(--text3)', 'margin-right': '4px' }}>steps</span>
          <span style={{ color: 'var(--cyan)', 'font-weight': 700 }}>
            {formatNumber(props.data.metrics.steps_taken)}
          </span>
        </div>
        <div>
          <span style={{ color: 'var(--text3)', 'margin-right': '4px' }}>visited</span>
          <span style={{ color: 'var(--purple)', 'font-weight': 700 }}>
            {formatNumber(props.data.metrics.cells_visited)}
          </span>
        </div>
        <div>
          <span style={{ color: 'var(--text3)', 'margin-right': '4px' }}>path</span>
          <span style={{ color: 'var(--green)', 'font-weight': 700 }}>
            {props.data.metrics.path_length || '\u2014'}
          </span>
        </div>
      </div>
    </div>
  );
};

interface ComparisonViewProps {
  data: ComparisonSolveData[];
  width: number;
  height: number;
  topology?: string;
}

const ComparisonView: Component<ComparisonViewProps> = (props) => {
  const winnerIndex = (): number => {
    // Find shortest path length among all (ignoring 0 = not found yet)
    let bestPath = Infinity;
    for (const d of props.data) {
      const p = d.metrics.path_length;
      if (p > 0 && p < bestPath) bestPath = p;
    }
    if (bestPath === Infinity) return -1; // no one found a path yet

    // Count how many have this best path
    const withBest = props.data
      .map((d, i) => ({ i, p: d.metrics.path_length, s: d.metrics.steps_taken }))
      .filter(x => x.p === bestPath);

    if (withBest.length === 0) return -1;
    if (withBest.length === 1) return withBest[0].i;

    // Multiple with same path length -- pick fewest steps
    withBest.sort((a, b) => a.s - b.s);
    // If top 2 have same steps too -- tie, no winner
    if (withBest[0].s === withBest[1].s) return -1;
    return withBest[0].i;
  };

  const count = () => props.data.length;
  // Grid layout: up to 4 columns, rows auto
  const cols = () => count() <= 2 ? count() : count() <= 4 ? 2 : count() <= 6 ? 3 : 4;
  const rows = () => Math.ceil(count() / cols());

  return (
    <div style={{
      width: '100%',
      height: '100%',
      display: 'grid',
      'grid-template-columns': `repeat(${cols()}, 1fr)`,
      'grid-template-rows': `repeat(${rows()}, 1fr)`,
      gap: '2px',
      background: 'var(--border)',
      overflow: 'hidden',
    }}>
      {props.data.map((d, i) => (
        <div style={{ overflow: 'hidden', background: 'var(--bg)', 'min-height': '0' }}>
          <ComparisonPane
            data={d}
            width={props.width}
            height={props.height}
            isWinner={winnerIndex() === i}
            topology={props.topology}
          />
        </div>
      ))}
    </div>
  );
};

export default ComparisonView;
