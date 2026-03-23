import { Component, createEffect, createSignal, onMount, onCleanup } from 'solid-js';
import type { MazeState } from '../../stores/maze';
import { t, locale } from '../../i18n';
import * as wasm from '../../wasm/bridge';
import { WALL_N, WALL_E, WALL_S, WALL_W, CELL_COLORS, renderMazeToCanvas } from '../../rendering/mazeRenderer';
import type { MazeRenderData } from '../../rendering/mazeRenderer';

function heatColor(t: number): string {
  // t: 0..1
  const r = Math.min(255, Math.floor(t < 0.5 ? t * 2 * 255 : 255));
  const g = Math.min(255, Math.floor(t < 0.5 ? t * 2 * 200 : (1 - t) * 2 * 200));
  const b = Math.min(255, Math.floor(t < 0.5 ? (1 - t * 2) * 255 : 0));
  return `rgba(${r},${g},${b},0.55)`;
}

interface ViewTransform {
  offsetX: number;
  offsetY: number;
  scale: number;
}

// Direction bits matching bridge.ts: 0=N, 1=E, 2=S, 3=W
const DIR_N = 0;
const DIR_E = 1;
const DIR_S = 2;
const DIR_W = 3;

/** Convert a direction index (0-3) to its wall bitmask */
function dirToWallBit(dir: number): number {
  return 1 << dir; // 0->0b0001, 1->0b0010, 2->0b0100, 3->0b1000
}

const MazeCanvas: Component<{ store: MazeState; canvasRef?: (el: HTMLCanvasElement) => void }> = (props) => {
  let canvasRef: HTMLCanvasElement | undefined;
  let containerRef: HTMLDivElement | undefined;

  const [viewTransform, setViewTransform] = createSignal<ViewTransform>({
    offsetX: 0,
    offsetY: 0,
    scale: 1,
  });

  // Pan state
  let isPanning = false;
  let panStartX = 0;
  let panStartY = 0;
  let panStartOffsetX = 0;
  let panStartOffsetY = 0;

  // Wall draw/erase drag state
  let isDrawing = false;
  let lastWallAction: { cell: number; dir: number } | null = null;

  const resetView = () => {
    setViewTransform({ offsetX: 0, offsetY: 0, scale: 1 });
  };

  /** Compute maze layout metrics (shared by render and hit-test). */
  const getMazeLayout = () => {
    const container = containerRef;
    if (!container) return null;
    const displayWidth = container.clientWidth;
    const displayHeight = container.clientHeight;
    const w = props.store.activeWidth();
    const h = props.store.activeHeight();

    const padding = 20;
    const availW = displayWidth - 2 * padding;
    const availH = displayHeight - 2 * padding;
    const cellSize = Math.max(2, Math.min(availW / w, availH / h));
    const mazeW = cellSize * w;
    const mazeH = cellSize * h;
    const baseOffsetX = (displayWidth - mazeW) / 2;
    const baseOffsetY = (displayHeight - mazeH) / 2;

    return { displayWidth, displayHeight, w, h, cellSize, mazeW, mazeH, baseOffsetX, baseOffsetY };
  };

  /** Convert screen (client) coordinates to maze cell coordinates. */
  const screenToMaze = (clientX: number, clientY: number): { col: number; row: number; localX: number; localY: number } | null => {
    const container = containerRef;
    if (!container) return null;
    const layout = getMazeLayout();
    if (!layout) return null;

    const rect = container.getBoundingClientRect();
    const sx = clientX - rect.left;
    const sy = clientY - rect.top;

    const vt = viewTransform();
    const tx = layout.baseOffsetX + vt.offsetX + layout.mazeW / 2 * (1 - vt.scale);
    const ty = layout.baseOffsetY + vt.offsetY + layout.mazeH / 2 * (1 - vt.scale);

    const mazeX = (sx - tx) / vt.scale;
    const mazeY = (sy - ty) / vt.scale;

    const col = Math.floor(mazeX / layout.cellSize);
    const row = Math.floor(mazeY / layout.cellSize);

    if (col < 0 || col >= layout.w || row < 0 || row >= layout.h) return null;

    const localX = mazeX - col * layout.cellSize;
    const localY = mazeY - row * layout.cellSize;

    return { col, row, localX, localY };
  };

  /** Detect which wall direction the pointer is closest to within a cell. Returns null if in center. */
  const detectWallDirection = (localX: number, localY: number, cellSize: number): number | null => {
    const ratioX = localX / cellSize;
    const ratioY = localY / cellSize;

    if (ratioY < 0.25) return DIR_N;
    if (ratioY > 0.75) return DIR_S;
    if (ratioX < 0.25) return DIR_W;
    if (ratioX > 0.75) return DIR_E;

    return null; // center zone -- no wall
  };

  /** Toggle a wall via WASM and refresh store. */
  const applyWallEdit = (cell: number, dir: number, mode: 'draw' | 'erase') => {
    const wallData = props.store.wallData();
    if (!wallData) return;

    const currentWalls = wallData[cell];
    const bit = dirToWallBit(dir);
    const hasWall = (currentWalls & bit) !== 0;

    // Only toggle if needed: draw adds, erase removes
    if (mode === 'draw' && hasWall) return;
    if (mode === 'erase' && !hasWall) return;

    wasm.toggleWall(cell, dir);

    // Refresh wall data from WASM
    const newWallData = wasm.getWallData(props.store.activeWidth(), props.store.activeHeight());
    props.store.setWallData(newWallData);
  };

  // =========================================================================
  // Main render dispatcher
  // =========================================================================
  const renderMaze = () => {
    const canvas = canvasRef;
    const container = containerRef;
    if (!canvas || !container) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const displayWidth = container.clientWidth;
    const displayHeight = container.clientHeight;

    // Set canvas buffer size for HiDPI
    canvas.width = displayWidth * dpr;
    canvas.height = displayHeight * dpr;

    // Scale context to match device pixels
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    // Clear background
    ctx.fillStyle = '#06080d';
    ctx.fillRect(0, 0, displayWidth, displayHeight);

    const wallData = props.store.wallData();
    const solutionPath = props.store.solutionPath();
    const cellStates = props.store.cellStates();
    const w = props.store.activeWidth();
    const h = props.store.activeHeight();
    const start = props.store.startCell();
    const end = props.store.endCell();
    const topology = props.store.topology();

    if (!wallData || wallData.length === 0) {
      ctx.fillStyle = '#484f58';
      ctx.font = '14px "JetBrains Mono", monospace';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(t('emptyState'), displayWidth / 2, displayHeight / 2);
      return;
    }

    const vt = viewTransform();

    // Get cell positions for non-rect topologies
    const cellPositions = topology !== 'rectangular' ? wasm.getCellPositions() : undefined;

    // Build render data
    const renderData: MazeRenderData = {
      wallData,
      cellStates,
      solutionPath,
      w,
      h,
      startCell: start,
      endCell: end,
      topology,
      cellPositions,
    };

    // Apply zoom/pan transform based on topology
    if (topology === 'rectangular') {
      const padding = 20;
      const availW = displayWidth - 2 * padding;
      const availH = displayHeight - 2 * padding;
      const cellSize = Math.max(2, Math.min(availW / w, availH / h));
      const mazeW = cellSize * w;
      const mazeH = cellSize * h;
      const baseOffsetX = (displayWidth - mazeW) / 2;
      const baseOffsetY = (displayHeight - mazeH) / 2;

      ctx.save();
      ctx.translate(
        baseOffsetX + vt.offsetX + mazeW / 2 * (1 - vt.scale),
        baseOffsetY + vt.offsetY + mazeH / 2 * (1 - vt.scale),
      );
      ctx.scale(vt.scale, vt.scale);

      // Use renderMazeToCanvas with displayWidth/Height set to mazeW/mazeH
      // so it fits exactly in the transformed coordinate space.
      // For rect, we need to render at the maze dimensions directly since
      // we've already positioned via translate.
      const solutionSet = new Set(solutionPath);

      // Draw cells
      for (let row = 0; row < h; row++) {
        for (let col = 0; col < w; col++) {
          const idx = row * w + col;
          const x = col * cellSize;
          const y = row * cellSize;
          if (idx === start) {
            ctx.fillStyle = '#00d4aa';
          } else if (idx === end) {
            ctx.fillStyle = '#ff6b6b';
          } else if (solutionSet.has(idx)) {
            ctx.fillStyle = CELL_COLORS[4]; // CELL_SOLUTION
          } else if (cellStates && idx < cellStates.length) {
            ctx.fillStyle = CELL_COLORS[cellStates[idx]] || CELL_COLORS[0];
          } else {
            ctx.fillStyle = CELL_COLORS[0]; // CELL_UNVISITED
          }
          ctx.fillRect(x, y, cellSize, cellSize);
        }
      }

      // Heatmap overlay (only for rectangular, only in main canvas)
      if (props.store.heatmapEnabled()) {
        const visitOrder = wasm.getVisitOrder(w, h);
        let maxOrder = 0;
        for (let i = 0; i < visitOrder.length; i++) {
          if (visitOrder[i] > maxOrder) maxOrder = visitOrder[i];
        }
        if (maxOrder > 0) {
          for (let row = 0; row < h; row++) {
            for (let col = 0; col < w; col++) {
              const idx = row * w + col;
              if (idx === start || idx === end) continue;
              const order = visitOrder[idx];
              if (order > 0) {
                const tVal = order / maxOrder;
                ctx.fillStyle = heatColor(tVal);
                ctx.fillRect(col * cellSize, row * cellSize, cellSize, cellSize);
              }
            }
          }
        }
      }

      // Walls
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

      // Solution path
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

      // Border
      ctx.save();
      ctx.strokeStyle = '#00d4aa';
      ctx.lineWidth = 2;
      ctx.shadowColor = 'rgba(0,212,170,0.3)';
      ctx.shadowBlur = 12;
      ctx.strokeRect(0, 0, mazeW, mazeH);
      ctx.restore();

      ctx.restore();
    } else {
      // Non-rect topologies: apply zoom/pan around center then delegate to shared renderer
      ctx.save();
      const cx = displayWidth / 2;
      const cy = displayHeight / 2;
      ctx.translate(cx + vt.offsetX, cy + vt.offsetY);
      ctx.scale(vt.scale, vt.scale);
      ctx.translate(-cx, -cy);

      renderMazeToCanvas(ctx, renderData, displayWidth, displayHeight);

      ctx.restore();
    }
  };

  // Zoom handler (zoom toward cursor position)
  const handleWheel = (e: WheelEvent) => {
    e.preventDefault();
    const container = containerRef;
    if (!container) return;

    const vt = viewTransform();
    const rawFactor = e.deltaY < 0 ? 1.1 : 1 / 1.1;
    const newScale = Math.max(0.1, Math.min(10, vt.scale * rawFactor));

    // Don't update offset if scale didn't change (hit the limit)
    if (newScale === vt.scale) return;

    const actualFactor = newScale / vt.scale;

    const rect = container.getBoundingClientRect();
    const mouseX = e.clientX - rect.left;
    const mouseY = e.clientY - rect.top;
    const centerX = container.clientWidth / 2;
    const centerY = container.clientHeight / 2;

    const dx = mouseX - centerX - vt.offsetX;
    const dy = mouseY - centerY - vt.offsetY;

    setViewTransform({
      offsetX: vt.offsetX - dx * (actualFactor - 1),
      offsetY: vt.offsetY - dy * (actualFactor - 1),
      scale: newScale,
    });
  };

  // Pointer handlers -- tool-mode aware
  const handlePointerDown = (e: PointerEvent) => {
    const mode = props.store.toolMode();

    // Middle mouse (1), right mouse (2), or spacebar held always pans
    const forcePan = e.button === 1 || e.button === 2 || spaceHeld;
    if (e.button !== 0 && !forcePan) return;

    if (mode === 'pan' || forcePan) {
      isPanning = true;
      panStartX = e.clientX;
      panStartY = e.clientY;
      const vt = viewTransform();
      panStartOffsetX = vt.offsetX;
      panStartOffsetY = vt.offsetY;
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
      return;
    }

    if (mode === 'draw' || mode === 'erase') {
      // Wall editing only supported for rectangular topology
      if (props.store.topology() !== 'rectangular') return;
      isDrawing = true;
      lastWallAction = null;
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
      handleWallInteraction(e, mode);
      return;
    }

    if (mode === 'set-start' || mode === 'set-end') {
      // Only rectangular topology supports click-to-set for now
      if (props.store.topology() !== 'rectangular') return;
      const hit = screenToMaze(e.clientX, e.clientY);
      if (!hit) return;
      const w = props.store.activeWidth();
      const cellIndex = hit.row * w + hit.col;
      if (mode === 'set-start') {
        props.store.setStartCell(cellIndex);
      } else {
        props.store.setEndCell(cellIndex);
      }
      return;
    }
  };

  const handleWallInteraction = (e: PointerEvent, mode: 'draw' | 'erase') => {
    const layout = getMazeLayout();
    if (!layout) return;
    const hit = screenToMaze(e.clientX, e.clientY);
    if (!hit) return;

    const dir = detectWallDirection(hit.localX, hit.localY, layout.cellSize);
    if (dir === null) return;

    const w = props.store.activeWidth();
    const cellIndex = hit.row * w + hit.col;

    // Avoid re-toggling the same wall on drag
    if (lastWallAction && lastWallAction.cell === cellIndex && lastWallAction.dir === dir) return;

    lastWallAction = { cell: cellIndex, dir };
    applyWallEdit(cellIndex, dir, mode);
  };

  const handlePointerMove = (e: PointerEvent) => {
    const mode = props.store.toolMode();

    if (isPanning) {
      const dx = e.clientX - panStartX;
      const dy = e.clientY - panStartY;
      setViewTransform((prev) => ({
        ...prev,
        offsetX: panStartOffsetX + dx,
        offsetY: panStartOffsetY + dy,
      }));
      return;
    }

    if ((mode === 'draw' || mode === 'erase') && isDrawing) {
      handleWallInteraction(e, mode);
      return;
    }
  };

  const handlePointerUp = (e: PointerEvent) => {
    if (isPanning) {
      isPanning = false;
      (e.target as HTMLElement).releasePointerCapture(e.pointerId);
    }
    if (isDrawing) {
      isDrawing = false;
      lastWallAction = null;
      (e.target as HTMLElement).releasePointerCapture(e.pointerId);
    }
  };

  // Spacebar pan support — temporarily switches tool to 'pan'
  let spaceHeld = false;
  let toolBeforeSpace: string | null = null;

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.code === 'Space' && !spaceHeld) {
      e.preventDefault();
      spaceHeld = true;
      toolBeforeSpace = props.store.toolMode();
      props.store.setToolMode('pan');
    }
  };

  const handleKeyUp = (e: KeyboardEvent) => {
    if (e.code === 'Space') {
      e.preventDefault();
      spaceHeld = false;
      if (toolBeforeSpace) {
        props.store.setToolMode(toolBeforeSpace as any);
        toolBeforeSpace = null;
      }
    }
  };

  /** Compute the CSS cursor based on tool mode. */
  const getCursor = (): string => {
    if (spaceHeld || isPanning) return 'grabbing';
    const mode = props.store.toolMode();
    if (mode === 'draw' || mode === 'erase') return 'crosshair';
    if (mode === 'set-start' || mode === 'set-end') return 'pointer';
    return 'grab';
  };

  onMount(() => {
    if (canvasRef) {
      props.canvasRef?.(canvasRef);
    }
    const observer = new ResizeObserver(() => renderMaze());
    if (containerRef) observer.observe(containerRef);
    renderMaze();
    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);
    onCleanup(() => {
      observer.disconnect();
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
    });
  });

  // React to data changes
  createEffect(() => {
    props.store.wallData();
    props.store.solutionPath();
    props.store.cellStates();
    props.store.startCell();
    props.store.endCell();
    props.store.heatmapEnabled();
    props.store.topology();
    viewTransform();
    locale();
    renderMaze();
  });

  return (
    <div
      ref={containerRef}
      onWheel={handleWheel}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onDblClick={resetView}
      onContextMenu={(e) => e.preventDefault()}
      style={{
        width: '100%',
        height: '100%',
        background: 'var(--bg)',
        cursor: getCursor(),
        'touch-action': 'none',
        position: 'relative',
        display: 'flex',
        'align-items': 'center',
        'justify-content': 'center',
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
      {/* Center/reset view button */}
      {viewTransform().scale !== 1 || viewTransform().offsetX !== 0 || viewTransform().offsetY !== 0 ? (
        <button
          onClick={resetView}
          title="Reset view"
          style={{
            position: 'absolute',
            top: '8px',
            right: '8px',
            width: '32px',
            height: '32px',
            'border-radius': '6px',
            background: 'rgba(13,17,23,0.85)',
            border: '1px solid var(--border)',
            color: 'var(--cyan)',
            'font-size': '16px',
            cursor: 'pointer',
            display: 'flex',
            'align-items': 'center',
            'justify-content': 'center',
            'z-index': '5',
            transition: 'all 0.15s',
          }}
        >
          {'\u2316'}
        </button>
      ) : null}
    </div>
  );
};

export default MazeCanvas;
