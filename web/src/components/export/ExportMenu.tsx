import { Component, Show } from 'solid-js';
import type { MazeState } from '../../stores/maze';
import { t } from '../../i18n';

// Note: PNG and SVG export currently only support rectangular topology.
// For hex/triangle/circular, the export uses the canvas screenshot approach.

const WALL_N = 0b0001;
const WALL_E = 0b0010;
const WALL_S = 0b0100;
const WALL_W = 0b1000;

// Cell state constants
const CELL_UNVISITED = 0;
const CELL_SOLUTION = 4;

// Cell state colors (same as MazeCanvas)
const CELL_COLORS: Record<number, string> = {
  0: '#0a0e14',
  1: 'rgba(88,166,255,0.15)',
  2: 'rgba(123,97,255,0.12)',
  3: 'rgba(0,212,170,0.25)',
  4: 'rgba(52,211,153,0.4)',
  5: 'rgba(255,75,75,0.08)',
};

function renderMazeToCanvas(store: MazeState, scale: number): HTMLCanvasElement | null {
  const wallData = store.wallData();
  const solutionPath = store.solutionPath();
  const cellStates = store.cellStates();
  const w = store.width;
  const h = store.height;

  if (!wallData || wallData.length === 0) return null;

  const baseCellSize = Math.max(8, Math.min(40, 800 / Math.max(w, h)));
  const cellSize = baseCellSize * scale;
  const padding = 20 * scale;

  const canvasWidth = cellSize * w + 2 * padding;
  const canvasHeight = cellSize * h + 2 * padding;

  const canvas = document.createElement('canvas');
  canvas.width = canvasWidth;
  canvas.height = canvasHeight;
  const ctx = canvas.getContext('2d');
  if (!ctx) return null;

  // Background
  ctx.fillStyle = '#06080d';
  ctx.fillRect(0, 0, canvasWidth, canvasHeight);

  ctx.save();
  ctx.translate(padding, padding);

  const solutionSet = new Set(solutionPath);

  // Draw cells
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

  // Draw walls
  const wallPath = new Path2D();
  for (let row = 0; row < h; row++) {
    for (let col = 0; col < w; col++) {
      const idx = row * w + col;
      const walls = wallData[idx];
      const x = col * cellSize;
      const y = row * cellSize;

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

  // Bottom border
  for (let col = 0; col < w; col++) {
    const idx = (h - 1) * w + col;
    const walls = wallData[idx];
    if (walls & WALL_S) {
      const x = col * cellSize;
      const y = h * cellSize;
      wallPath.moveTo(x, y);
      wallPath.lineTo(x + cellSize, y);
    }
  }

  // Right border
  for (let row = 0; row < h; row++) {
    const idx = row * w + (w - 1);
    const walls = wallData[idx];
    if (walls & WALL_E) {
      const x = w * cellSize;
      const y = row * cellSize;
      wallPath.moveTo(x, y);
      wallPath.lineTo(x, y + cellSize);
    }
  }

  ctx.strokeStyle = '#1a3355';
  ctx.lineWidth = Math.max(1, cellSize > 10 ? 1.5 * scale : scale);
  ctx.stroke(wallPath);

  // Draw solution path line
  if (solutionPath.length > 1 && cellSize >= 3) {
    const pathLineWidth = Math.max(1.5, cellSize / 4);

    ctx.save();
    ctx.strokeStyle = '#34d399';
    ctx.lineWidth = pathLineWidth;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.shadowColor = '#34d399';
    ctx.shadowBlur = 8 * scale;

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

  // Outer border with glow
  ctx.save();
  ctx.strokeStyle = '#00d4aa';
  ctx.lineWidth = 2 * scale;
  ctx.shadowColor = 'rgba(0,212,170,0.3)';
  ctx.shadowBlur = 12 * scale;
  ctx.strokeRect(0, 0, cellSize * w, cellSize * h);
  ctx.restore();

  ctx.restore();

  return canvas;
}

function downloadCanvas(canvas: HTMLCanvasElement, filename: string): void {
  canvas.toBlob((blob) => {
    if (!blob) return;
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }, 'image/png');
}

function generateMazeSvg(store: MazeState): string | null {
  const wallData = store.wallData();
  const solutionPath = store.solutionPath();
  const cellStates = store.cellStates();
  const w = store.width;
  const h = store.height;

  if (!wallData || wallData.length === 0) return null;

  const cellSize = Math.max(8, Math.min(40, 800 / Math.max(w, h)));
  const padding = 20;
  const svgWidth = cellSize * w + 2 * padding;
  const svgHeight = cellSize * h + 2 * padding;

  const solutionSet = new Set(solutionPath);

  const parts: string[] = [];
  parts.push(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${svgWidth} ${svgHeight}" width="${svgWidth}" height="${svgHeight}">`);

  // Background
  parts.push(`<rect width="${svgWidth}" height="${svgHeight}" fill="#06080d"/>`);

  // Group offset by padding
  parts.push(`<g transform="translate(${padding},${padding})">`);

  // Draw cells
  for (let row = 0; row < h; row++) {
    for (let col = 0; col < w; col++) {
      const idx = row * w + col;
      const x = col * cellSize;
      const y = row * cellSize;

      let fill: string;
      if (idx === 0) {
        fill = '#00d4aa';
      } else if (idx === w * h - 1) {
        fill = '#ff6b6b';
      } else if (solutionSet.has(idx)) {
        fill = CELL_COLORS[CELL_SOLUTION];
      } else if (cellStates && idx < cellStates.length) {
        fill = CELL_COLORS[cellStates[idx]] || CELL_COLORS[CELL_UNVISITED];
      } else {
        fill = CELL_COLORS[CELL_UNVISITED];
      }
      parts.push(`<rect x="${x}" y="${y}" width="${cellSize}" height="${cellSize}" fill="${fill}"/>`);
    }
  }

  // Draw walls (N and W per cell + bottom/right borders)
  const wallLineWidth = cellSize > 10 ? 1.5 : 1;
  const wallLines: string[] = [];

  for (let row = 0; row < h; row++) {
    for (let col = 0; col < w; col++) {
      const idx = row * w + col;
      const walls = wallData[idx];
      const x = col * cellSize;
      const y = row * cellSize;

      if (walls & WALL_N) {
        wallLines.push(`<line x1="${x}" y1="${y}" x2="${x + cellSize}" y2="${y}"/>`);
      }
      if (walls & WALL_W) {
        wallLines.push(`<line x1="${x}" y1="${y}" x2="${x}" y2="${y + cellSize}"/>`);
      }
    }
  }

  // Bottom border
  for (let col = 0; col < w; col++) {
    const idx = (h - 1) * w + col;
    const walls = wallData[idx];
    if (walls & WALL_S) {
      const x = col * cellSize;
      const y = h * cellSize;
      wallLines.push(`<line x1="${x}" y1="${y}" x2="${x + cellSize}" y2="${y}"/>`);
    }
  }

  // Right border
  for (let row = 0; row < h; row++) {
    const idx = row * w + (w - 1);
    const walls = wallData[idx];
    if (walls & WALL_E) {
      const x = w * cellSize;
      const y = row * cellSize;
      wallLines.push(`<line x1="${x}" y1="${y}" x2="${x}" y2="${y + cellSize}"/>`);
    }
  }

  parts.push(`<g stroke="#1a3355" stroke-width="${wallLineWidth}" stroke-linecap="square">`);
  parts.push(...wallLines);
  parts.push('</g>');

  // Draw solution path
  if (solutionPath.length > 1) {
    const pathLineWidth = Math.max(1.5, cellSize / 4);
    const points = solutionPath.map((cell) => {
      const col = cell % w;
      const row = Math.floor(cell / w);
      const cx = col * cellSize + cellSize / 2;
      const cy = row * cellSize + cellSize / 2;
      return `${cx},${cy}`;
    }).join(' ');
    parts.push(`<polyline points="${points}" fill="none" stroke="#34d399" stroke-width="${pathLineWidth}" stroke-linecap="round" stroke-linejoin="round"/>`);
  }

  // Outer border
  parts.push(`<rect x="0" y="0" width="${cellSize * w}" height="${cellSize * h}" fill="none" stroke="#00d4aa" stroke-width="2"/>`);

  parts.push('</g>');
  parts.push('</svg>');

  return parts.join('\n');
}

function downloadSvg(content: string, filename: string): void {
  const blob = new Blob([content], { type: 'image/svg+xml' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

const ExportMenu: Component<{
  store: MazeState;
  onRecordVideo?: () => void;
  isRecording?: () => boolean;
}> = (props) => {
  const handleExportPng = () => {
    const canvas = renderMazeToCanvas(props.store, 3);
    if (!canvas) return;
    const w = props.store.width;
    const h = props.store.height;
    const filename = `maze_${w}x${h}_s${props.store.seed}.png`;
    downloadCanvas(canvas, filename);
  };

  const handleExportSvg = () => {
    const svg = generateMazeSvg(props.store);
    if (!svg) return;
    const w = props.store.width;
    const h = props.store.height;
    const filename = `maze_${w}x${h}_s${props.store.seed}.svg`;
    downloadSvg(svg, filename);
  };

  const hasData = () => !!props.store.wallData();
  const recording = () => props.isRecording?.() ?? false;

  return (
    <div style={{
      padding: '14px 16px',
      'border-bottom': '1px solid rgba(26,39,68,0.5)',
    }}>
      <div style={{
        'font-size': '10px',
        'text-transform': 'uppercase',
        'letter-spacing': '0.12em',
        color: 'var(--text3)',
        'font-family': 'var(--font-mono)',
        'margin-bottom': '10px',
      }}>
        {t('export')}
      </div>
      <div style={{ display: 'flex', 'flex-direction': 'column', gap: '6px' }}>
        <button
          onClick={handleExportPng}
          disabled={!hasData()}
          style={{
            padding: '7px 0',
            'border-radius': '6px',
            'font-size': '12px',
            'font-weight': 600,
            cursor: hasData() ? 'pointer' : 'not-allowed',
            transition: 'all 0.15s',
            'text-align': 'center',
            width: '100%',
            'font-family': 'var(--font-sans)',
            background: hasData() ? 'rgba(240,180,41,0.1)' : 'var(--bg)',
            border: hasData() ? '1px solid var(--amber)' : '1px solid var(--border)',
            color: hasData() ? 'var(--amber)' : 'var(--text3)',
            opacity: hasData() ? '1' : '0.5',
          }}
        >
          {t('savePng')}
        </button>
        <button
          onClick={handleExportSvg}
          disabled={!hasData()}
          style={{
            padding: '7px 0',
            'border-radius': '6px',
            'font-size': '12px',
            'font-weight': 600,
            cursor: hasData() ? 'pointer' : 'not-allowed',
            transition: 'all 0.15s',
            'text-align': 'center',
            width: '100%',
            'font-family': 'var(--font-sans)',
            background: hasData() ? 'rgba(255,160,40,0.1)' : 'var(--bg)',
            border: hasData() ? '1px solid #e09422' : '1px solid var(--border)',
            color: hasData() ? '#e09422' : 'var(--text3)',
            opacity: hasData() ? '1' : '0.5',
          }}
        >
          {t('saveSvg')}
        </button>
        <button
          onClick={() => props.onRecordVideo?.()}
          disabled={!hasData() || recording()}
          style={{
            padding: '7px 0',
            'border-radius': '6px',
            'font-size': '12px',
            'font-weight': 600,
            cursor: hasData() && !recording() ? 'pointer' : 'not-allowed',
            transition: 'all 0.15s',
            'text-align': 'center',
            width: '100%',
            'font-family': 'var(--font-sans)',
            background: recording()
              ? 'rgba(255,107,107,0.15)'
              : hasData()
                ? 'rgba(255,107,107,0.1)'
                : 'var(--bg)',
            border: recording()
              ? '1px solid var(--red)'
              : hasData()
                ? '1px solid var(--red)'
                : '1px solid var(--border)',
            color: recording()
              ? 'var(--red)'
              : hasData()
                ? 'var(--red)'
                : 'var(--text3)',
            opacity: hasData() ? '1' : '0.5',
          }}
        >
          <Show when={recording()} fallback={t('recordVideo')}>
            <span style={{ display: 'inline-flex', 'align-items': 'center', gap: '6px' }}>
              <span style={{
                width: '8px',
                height: '8px',
                'border-radius': '50%',
                background: 'var(--red)',
                display: 'inline-block',
                animation: 'pulse-dot 1s ease-in-out infinite',
              }} />
              {t('recording')}
            </span>
          </Show>
        </button>
      </div>
      <style>{`
        @keyframes pulse-dot {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.3; }
        }
      `}</style>
    </div>
  );
};

export default ExportMenu;
