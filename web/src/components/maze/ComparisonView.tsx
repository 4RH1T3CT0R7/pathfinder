import { Component, createEffect, createSignal, onMount, onCleanup } from 'solid-js';
import type { ComparisonSolveData } from '../../stores/maze';

const WALL_N = 0b0001;
const WALL_E = 0b0010;
const WALL_S = 0b0100;
const WALL_W = 0b1000;

const CELL_UNVISITED = 0;
const CELL_FRONTIER = 1;
const CELL_VISITED = 2;
const CELL_ACTIVE = 3;
const CELL_SOLUTION = 4;
const CELL_BACKTRACKED = 5;

const CELL_COLORS: Record<number, string> = {
  [CELL_UNVISITED]: '#0a0e14',
  [CELL_FRONTIER]: 'rgba(88,166,255,0.15)',
  [CELL_VISITED]: 'rgba(123,97,255,0.12)',
  [CELL_ACTIVE]: 'rgba(0,212,170,0.25)',
  [CELL_SOLUTION]: 'rgba(52,211,153,0.4)',
  [CELL_BACKTRACKED]: 'rgba(255,75,75,0.08)',
};

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
}

const ComparisonPane: Component<PaneProps> = (props) => {
  let canvasRef: HTMLCanvasElement | undefined;
  let containerRef: HTMLDivElement | undefined;

  const renderMaze = () => {
    const canvas = canvasRef;
    const container = containerRef;
    if (!canvas || !container) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const displayWidth = container.clientWidth;
    const displayHeight = container.clientHeight;

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

    if (!wallData || wallData.length === 0) return;

    const padding = 16;
    const availW = displayWidth - 2 * padding;
    const availH = displayHeight - 2 * padding;
    const cellSize = Math.max(2, Math.min(availW / w, availH / h));
    const mazeW = cellSize * w;
    const mazeH = cellSize * h;
    const baseOffsetX = (displayWidth - mazeW) / 2;
    const baseOffsetY = (displayHeight - mazeH) / 2;

    ctx.save();
    ctx.translate(baseOffsetX, baseOffsetY);

    const solutionSet = new Set(solutionPath);

    for (let row = 0; row < h; row++) {
      for (let col = 0; col < w; col++) {
        const idx = row * w + col;
        const x = col * cellSize;
        const y = row * cellSize;

        if (idx === 0) {
          ctx.fillStyle = '#00d4aa';
        } else if (idx === w * h - 1) {
          ctx.fillStyle = '#ff6b6b';
        } else if (solutionSet.has(idx)) {
          ctx.fillStyle = CELL_COLORS[CELL_SOLUTION];
        } else if (cellStates && idx < cellStates.length) {
          ctx.fillStyle = CELL_COLORS[cellStates[idx]] || CELL_COLORS[CELL_UNVISITED];
        } else {
          ctx.fillStyle = CELL_COLORS[CELL_UNVISITED];
        }
        ctx.fillRect(x, y, cellSize, cellSize);
      }
    }

    const wallPath = new Path2D();
    for (let row = 0; row < h; row++) {
      for (let col = 0; col < w; col++) {
        const idx = row * w + col;
        const walls = wallData[idx];
        const x = col * cellSize + 0.5;
        const y = row * cellSize + 0.5;

        if (walls & WALL_N) {
          wallPath.moveTo(x, y);
          wallPath.lineTo(x + cellSize, y);
        }
        if (walls & WALL_W) {
          wallPath.moveTo(x, y);
          wallPath.lineTo(x, y + cellSize);
        }
      }
    }

    for (let col = 0; col < w; col++) {
      const idx = (h - 1) * w + col;
      const walls = wallData[idx];
      if (walls & WALL_S) {
        const x = col * cellSize + 0.5;
        const y = h * cellSize + 0.5;
        wallPath.moveTo(x, y);
        wallPath.lineTo(x + cellSize, y);
      }
    }

    for (let row = 0; row < h; row++) {
      const idx = row * w + (w - 1);
      const walls = wallData[idx];
      if (walls & WALL_E) {
        const x = w * cellSize + 0.5;
        const y = row * cellSize + 0.5;
        wallPath.moveTo(x, y);
        wallPath.lineTo(x, y + cellSize);
      }
    }

    ctx.strokeStyle = '#1a3355';
    ctx.lineWidth = cellSize > 10 ? 1.5 : 1;
    ctx.stroke(wallPath);

    if (solutionPath.length > 1 && cellSize >= 3) {
      const pathLineWidth = Math.max(1.5, cellSize / 4);
      ctx.save();
      ctx.strokeStyle = '#34d399';
      ctx.lineWidth = pathLineWidth;
      ctx.lineCap = 'round';
      ctx.lineJoin = 'round';
      ctx.shadowColor = '#34d399';
      ctx.shadowBlur = 8;

      ctx.beginPath();
      for (let i = 0; i < solutionPath.length; i++) {
        const cell = solutionPath[i];
        const col = cell % w;
        const row = Math.floor(cell / w);
        const cx = col * cellSize + cellSize / 2;
        const cy = row * cellSize + cellSize / 2;
        if (i === 0) ctx.moveTo(cx, cy);
        else ctx.lineTo(cx, cy);
      }
      ctx.stroke();
      ctx.restore();
    }

    ctx.save();
    ctx.strokeStyle = '#00d4aa';
    ctx.lineWidth = 2;
    ctx.shadowColor = 'rgba(0,212,170,0.3)';
    ctx.shadowBlur = 12;
    ctx.strokeRect(0, 0, mazeW, mazeH);
    ctx.restore();

    ctx.restore();
  };

  onMount(() => {
    const observer = new ResizeObserver(() => renderMaze());
    if (containerRef) observer.observe(containerRef);
    renderMaze();
    onCleanup(() => observer.disconnect());
  });

  createEffect(() => {
    // Track the reactive data to trigger re-renders
    void props.data;
    void props.width;
    void props.height;
    renderMaze();
  });

  return (
    <div style={{
      flex: '1',
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

      {/* Canvas */}
      <div
        ref={containerRef}
        style={{
          flex: '1',
          position: 'relative',
          background: 'var(--bg)',
          overflow: 'hidden',
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
  data: [ComparisonSolveData, ComparisonSolveData];
  width: number;
  height: number;
}

const ComparisonView: Component<ComparisonViewProps> = (props) => {
  const winner = (): 0 | 1 | -1 => {
    const [a, b] = props.data;
    const aPath = a.metrics.path_length;
    const bPath = b.metrics.path_length;
    // No path found by one or both
    if (aPath === 0 && bPath === 0) return -1;
    if (aPath === 0) return 1;
    if (bPath === 0) return 0;
    // Shorter path wins
    if (aPath < bPath) return 0;
    if (bPath < aPath) return 1;
    // Same path length -- fewer steps wins
    if (a.metrics.steps_taken < b.metrics.steps_taken) return 0;
    if (b.metrics.steps_taken < a.metrics.steps_taken) return 1;
    return -1; // tie
  };

  return (
    <div style={{
      width: '100%',
      height: '100%',
      display: 'flex',
      overflow: 'hidden',
    }}>
      <ComparisonPane
        data={props.data[0]}
        width={props.width}
        height={props.height}
        isWinner={winner() === 0}
      />
      <div style={{
        width: '2px',
        background: 'var(--border)',
        'flex-shrink': '0',
      }} />
      <ComparisonPane
        data={props.data[1]}
        width={props.width}
        height={props.height}
        isWinner={winner() === 1}
      />
    </div>
  );
};

export default ComparisonView;
