import { Component, createEffect, createSignal, onMount, onCleanup } from 'solid-js';
import type { MazeState } from '../../stores/maze';
import { t, locale } from '../../i18n';
import * as wasm from '../../wasm/bridge';

const WALL_N = 0b0001;
const WALL_E = 0b0010;
const WALL_S = 0b0100;
const WALL_W = 0b1000;

// Cell state constants
const CELL_UNVISITED = 0;
const CELL_FRONTIER = 1;
const CELL_VISITED = 2;
const CELL_ACTIVE = 3;
const CELL_SOLUTION = 4;
const CELL_BACKTRACKED = 5;

// Cell state colors
const CELL_COLORS: Record<number, string> = {
  [CELL_UNVISITED]: '#0a0e14',
  [CELL_FRONTIER]: 'rgba(88,166,255,0.15)',
  [CELL_VISITED]: 'rgba(123,97,255,0.12)',
  [CELL_ACTIVE]: 'rgba(0,212,170,0.25)',
  [CELL_SOLUTION]: 'rgba(52,211,153,0.4)',
  [CELL_BACKTRACKED]: 'rgba(255,75,75,0.08)',
};

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

/** Get cell fill color based on state. */
function getCellFillColor(
  idx: number,
  start: number,
  end: number,
  solutionSet: Set<number>,
  cellStates: Uint8Array | null,
): string {
  if (idx === start) return '#00d4aa';
  if (idx === end) return '#ff6b6b';
  if (solutionSet.has(idx)) return CELL_COLORS[CELL_SOLUTION];
  if (cellStates && idx < cellStates.length) {
    return CELL_COLORS[cellStates[idx]] || CELL_COLORS[CELL_UNVISITED];
  }
  return CELL_COLORS[CELL_UNVISITED];
}

// =============================================================================
// Hex rendering helpers
// =============================================================================

/** Compute the 6 vertices of a pointy-top hexagon centered at (cx, cy) with size s. */
function hexVertices(cx: number, cy: number, s: number): [number, number][] {
  const verts: [number, number][] = [];
  for (let i = 0; i < 6; i++) {
    const angle = Math.PI / 6 + i * Math.PI / 3;
    verts.push([cx + s * Math.cos(angle), cy + s * Math.sin(angle)]);
  }
  return verts;
}

// =============================================================================
// Triangle rendering helpers
// =============================================================================

/** Get the 3 vertices of a triangle cell. */
function triVertices(
  cx: number,
  cy: number,
  s: number,
  isUp: boolean,
): [number, number][] {
  const h = s * Math.sqrt(3) / 2;
  if (isUp) {
    return [
      [cx - s / 2, cy + h / 3],
      [cx + s / 2, cy + h / 3],
      [cx, cy - 2 * h / 3],
    ];
  } else {
    return [
      [cx - s / 2, cy - h / 3],
      [cx + s / 2, cy - h / 3],
      [cx, cy + 2 * h / 3],
    ];
  }
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
    const w = props.store.width;
    const h = props.store.height;

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
    const newWallData = wasm.getWallData(props.store.width, props.store.height);
    props.store.setWallData(newWallData);
  };

  // =========================================================================
  // Rectangular maze rendering
  // =========================================================================
  const renderRectMaze = (
    ctx: CanvasRenderingContext2D,
    wallData: Uint8Array,
    cellStates: Uint8Array | null,
    solutionPath: number[],
    solutionSet: Set<number>,
    w: number,
    h: number,
    start: number,
    end: number,
    cellSize: number,
    mazeW: number,
    mazeH: number,
  ) => {
    // Draw cells
    for (let row = 0; row < h; row++) {
      for (let col = 0; col < w; col++) {
        const idx = row * w + col;
        const x = col * cellSize;
        const y = row * cellSize;
        ctx.fillStyle = getCellFillColor(idx, start, end, solutionSet, cellStates);
        ctx.fillRect(x, y, cellSize, cellSize);
      }
    }

    // Heatmap overlay
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

    // Batch all walls into a single Path2D for performance
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

    // Draw bottom border walls (South walls of last row)
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

    // Draw right border walls (East walls of last column)
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

    // Draw solution path line with glow
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

    // Draw outer border with cyan glow
    ctx.save();
    ctx.strokeStyle = '#00d4aa';
    ctx.lineWidth = 2;
    ctx.shadowColor = 'rgba(0,212,170,0.3)';
    ctx.shadowBlur = 12;
    ctx.strokeRect(0, 0, mazeW, mazeH);
    ctx.restore();
  };

  // =========================================================================
  // Hexagonal maze rendering
  // =========================================================================
  const renderHexMaze = (
    ctx: CanvasRenderingContext2D,
    wallData: Uint8Array,
    cellStates: Uint8Array | null,
    solutionPath: number[],
    solutionSet: Set<number>,
    w: number,
    h: number,
    start: number,
    end: number,
    cellPositions: Float64Array,
    scaleFactor: number,
    offsetX: number,
    offsetY: number,
  ) => {
    const cellCount = wasm.getCellCount();
    const hexSize = scaleFactor;

    // Draw cell fills
    for (let i = 0; i < cellCount; i++) {
      const px = cellPositions[i * 2] * scaleFactor + offsetX;
      const py = cellPositions[i * 2 + 1] * scaleFactor + offsetY;
      const verts = hexVertices(px, py, hexSize);

      ctx.fillStyle = getCellFillColor(i, start, end, solutionSet, cellStates);
      ctx.beginPath();
      ctx.moveTo(verts[0][0], verts[0][1]);
      for (let v = 1; v < 6; v++) {
        ctx.lineTo(verts[v][0], verts[v][1]);
      }
      ctx.closePath();
      ctx.fill();
    }

    // Draw walls: each hex cell has 6 edges. Wall bitmask: E(0) NE(1) NW(2) W(3) SW(4) SE(5)
    // Edge i connects vertex i to vertex (i+1)%6 in our vertex array.
    // Our vertex array starts at angle PI/6 and goes counter-clockwise.
    // Hex edge mapping: direction bit i -> edge between vertices i and (i+1)%6
    const wallPath = new Path2D();
    for (let i = 0; i < cellCount; i++) {
      const walls = wallData[i];
      if (walls === 0) continue;
      const px = cellPositions[i * 2] * scaleFactor + offsetX;
      const py = cellPositions[i * 2 + 1] * scaleFactor + offsetY;
      const verts = hexVertices(px, py, hexSize);

      for (let d = 0; d < 6; d++) {
        if (walls & (1 << d)) {
          const v1 = verts[d];
          const v2 = verts[(d + 1) % 6];
          wallPath.moveTo(v1[0], v1[1]);
          wallPath.lineTo(v2[0], v2[1]);
        }
      }
    }

    ctx.strokeStyle = '#1a3355';
    ctx.lineWidth = hexSize > 10 ? 1.5 : 1;
    ctx.stroke(wallPath);

    // Draw solution path
    if (solutionPath.length > 1) {
      const pathLineWidth = Math.max(1.5, hexSize / 4);
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
        const cx = cellPositions[cell * 2] * scaleFactor + offsetX;
        const cy = cellPositions[cell * 2 + 1] * scaleFactor + offsetY;
        if (i === 0) ctx.moveTo(cx, cy);
        else ctx.lineTo(cx, cy);
      }
      ctx.stroke();
      ctx.restore();
    }
  };

  // =========================================================================
  // Triangular maze rendering
  // =========================================================================
  const renderTriMaze = (
    ctx: CanvasRenderingContext2D,
    wallData: Uint8Array,
    cellStates: Uint8Array | null,
    solutionPath: number[],
    solutionSet: Set<number>,
    w: number,
    h: number,
    start: number,
    end: number,
    cellPositions: Float64Array,
    scaleFactor: number,
    offsetX: number,
    offsetY: number,
  ) => {
    const cellCount = wasm.getCellCount();
    const triSide = scaleFactor;

    // Draw cell fills
    for (let i = 0; i < cellCount; i++) {
      const col = i % w;
      const row = Math.floor(i / w);
      const isUp = (col + row) % 2 === 0;
      const px = cellPositions[i * 2] * scaleFactor + offsetX;
      const py = cellPositions[i * 2 + 1] * scaleFactor + offsetY;
      const verts = triVertices(px, py, triSide, isUp);

      ctx.fillStyle = getCellFillColor(i, start, end, solutionSet, cellStates);
      ctx.beginPath();
      ctx.moveTo(verts[0][0], verts[0][1]);
      ctx.lineTo(verts[1][0], verts[1][1]);
      ctx.lineTo(verts[2][0], verts[2][1]);
      ctx.closePath();
      ctx.fill();
    }

    // Draw walls: LEFT(0) RIGHT(1) BASE(2)
    // Up triangle: LEFT=edge from bottom-left to top, RIGHT=edge from top to bottom-right, BASE=bottom edge
    // Down triangle: LEFT=edge from top-left to bottom, RIGHT=edge from bottom to top-right, BASE=top edge
    const wallPath = new Path2D();
    for (let i = 0; i < cellCount; i++) {
      const walls = wallData[i];
      if (walls === 0) continue;
      const col = i % w;
      const row = Math.floor(i / w);
      const isUp = (col + row) % 2 === 0;
      const px = cellPositions[i * 2] * scaleFactor + offsetX;
      const py = cellPositions[i * 2 + 1] * scaleFactor + offsetY;
      const verts = triVertices(px, py, triSide, isUp);

      // LEFT (bit 0): edge from verts[0] to verts[2] (bottom-left to apex for up)
      if (walls & 0b001) {
        wallPath.moveTo(verts[0][0], verts[0][1]);
        wallPath.lineTo(verts[2][0], verts[2][1]);
      }
      // RIGHT (bit 1): edge from verts[2] to verts[1] (apex to bottom-right for up)
      if (walls & 0b010) {
        wallPath.moveTo(verts[2][0], verts[2][1]);
        wallPath.lineTo(verts[1][0], verts[1][1]);
      }
      // BASE (bit 2): edge from verts[0] to verts[1] (bottom edge for up)
      if (walls & 0b100) {
        wallPath.moveTo(verts[0][0], verts[0][1]);
        wallPath.lineTo(verts[1][0], verts[1][1]);
      }
    }

    ctx.strokeStyle = '#1a3355';
    ctx.lineWidth = triSide > 10 ? 1.5 : 1;
    ctx.stroke(wallPath);

    // Draw solution path
    if (solutionPath.length > 1) {
      const pathLineWidth = Math.max(1.5, triSide / 4);
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
        const cx = cellPositions[cell * 2] * scaleFactor + offsetX;
        const cy = cellPositions[cell * 2 + 1] * scaleFactor + offsetY;
        if (i === 0) ctx.moveTo(cx, cy);
        else ctx.lineTo(cx, cy);
      }
      ctx.stroke();
      ctx.restore();
    }
  };

  // =========================================================================
  // Circular maze rendering
  // =========================================================================
  const renderCircularMaze = (
    ctx: CanvasRenderingContext2D,
    wallData: Uint8Array,
    cellStates: Uint8Array | null,
    solutionPath: number[],
    solutionSet: Set<number>,
    start: number,
    end: number,
    cellPositions: Float64Array,
    scaleFactor: number,
    centerX: number,
    centerY: number,
    rings: number,
  ) => {
    const cellCount = wasm.getCellCount();

    // Reconstruct ring structure: ring 0 has 1 cell, ring r has 6*r cells
    const cellsPerRing: number[] = [];
    const ringOffset: number[] = [];
    let totalCells = 0;
    for (let r = 0; r < rings; r++) {
      const cpr = r === 0 ? 1 : 6 * r;
      cellsPerRing.push(cpr);
      ringOffset.push(totalCells);
      totalCells += cpr;
    }

    const ringWidth = scaleFactor;

    // Draw cell fills using arc segments
    for (let i = 0; i < cellCount; i++) {
      // Determine ring and position from cell index
      let ring = 0;
      let pos = 0;
      for (let r = 0; r < rings; r++) {
        if (i < ringOffset[r] + cellsPerRing[r]) {
          ring = r;
          pos = i - ringOffset[r];
          break;
        }
      }

      ctx.fillStyle = getCellFillColor(i, start, end, solutionSet, cellStates);

      if (ring === 0) {
        // Center cell: draw a circle
        ctx.beginPath();
        ctx.arc(centerX, centerY, ringWidth, 0, Math.PI * 2);
        ctx.fill();
      } else {
        const cpr = cellsPerRing[ring];
        const angleStart = (pos / cpr) * Math.PI * 2 - Math.PI / 2;
        const angleEnd = ((pos + 1) / cpr) * Math.PI * 2 - Math.PI / 2;
        const innerR = ring * ringWidth;
        const outerR = (ring + 1) * ringWidth;

        ctx.beginPath();
        ctx.arc(centerX, centerY, innerR, angleStart, angleEnd);
        ctx.arc(centerX, centerY, outerR, angleEnd, angleStart, true);
        ctx.closePath();
        ctx.fill();
      }
    }

    // Draw walls
    // Circular wall directions: INWARD(0) CW(1) OUTWARD(2) CCW(3) OUTWARD2(4)
    const wallPath = new Path2D();
    for (let i = 0; i < cellCount; i++) {
      const walls = wallData[i];
      if (walls === 0) continue;

      let ring = 0;
      let pos = 0;
      for (let r = 0; r < rings; r++) {
        if (i < ringOffset[r] + cellsPerRing[r]) {
          ring = r;
          pos = i - ringOffset[r];
          break;
        }
      }

      if (ring === 0) {
        // Center cell: outward walls are drawn as radial lines at each 60-degree sector
        // Wall bits 0-5 correspond to the 6 outward edges
        const outerR = ringWidth;
        for (let d = 0; d < 6; d++) {
          if (walls & (1 << d)) {
            const angle = (d / 6) * Math.PI * 2 - Math.PI / 2;
            const nextAngle = ((d + 1) / 6) * Math.PI * 2 - Math.PI / 2;
            // Draw the arc segment on the outer ring boundary
            wallPath.moveTo(
              centerX + outerR * Math.cos(angle),
              centerY + outerR * Math.sin(angle),
            );
            wallPath.arc(centerX, centerY, outerR, angle, nextAngle);
          }
        }
      } else {
        const cpr = cellsPerRing[ring];
        const angleStart = (pos / cpr) * Math.PI * 2 - Math.PI / 2;
        const angleEnd = ((pos + 1) / cpr) * Math.PI * 2 - Math.PI / 2;
        const innerR = ring * ringWidth;
        const outerR = (ring + 1) * ringWidth;

        // INWARD (bit 0): inner arc
        if (walls & 0b00001) {
          wallPath.moveTo(
            centerX + innerR * Math.cos(angleStart),
            centerY + innerR * Math.sin(angleStart),
          );
          wallPath.arc(centerX, centerY, innerR, angleStart, angleEnd);
        }

        // CW (bit 1): right radial line (at angleEnd)
        if (walls & 0b00010) {
          wallPath.moveTo(
            centerX + innerR * Math.cos(angleEnd),
            centerY + innerR * Math.sin(angleEnd),
          );
          wallPath.lineTo(
            centerX + outerR * Math.cos(angleEnd),
            centerY + outerR * Math.sin(angleEnd),
          );
        }

        // OUTWARD (bit 2): outer arc (full cell width)
        if (walls & 0b00100) {
          wallPath.moveTo(
            centerX + outerR * Math.cos(angleStart),
            centerY + outerR * Math.sin(angleStart),
          );
          wallPath.arc(centerX, centerY, outerR, angleStart, angleEnd);
        }

        // CCW (bit 3): left radial line (at angleStart)
        if (walls & 0b01000) {
          wallPath.moveTo(
            centerX + innerR * Math.cos(angleStart),
            centerY + innerR * Math.sin(angleStart),
          );
          wallPath.lineTo(
            centerX + outerR * Math.cos(angleStart),
            centerY + outerR * Math.sin(angleStart),
          );
        }
      }
    }

    ctx.strokeStyle = '#1a3355';
    ctx.lineWidth = ringWidth > 10 ? 1.5 : 1;
    ctx.stroke(wallPath);

    // Draw outer border circle
    ctx.save();
    ctx.strokeStyle = '#00d4aa';
    ctx.lineWidth = 2;
    ctx.shadowColor = 'rgba(0,212,170,0.3)';
    ctx.shadowBlur = 12;
    ctx.beginPath();
    ctx.arc(centerX, centerY, rings * ringWidth, 0, Math.PI * 2);
    ctx.stroke();
    ctx.restore();

    // Draw solution path
    if (solutionPath.length > 1) {
      const pathLineWidth = Math.max(1.5, ringWidth / 4);
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
        const cx = cellPositions[cell * 2] * scaleFactor + centerX;
        const cy = cellPositions[cell * 2 + 1] * scaleFactor + centerY;
        if (i === 0) ctx.moveTo(cx, cy);
        else ctx.lineTo(cx, cy);
      }
      ctx.stroke();
      ctx.restore();
    }
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
    const w = props.store.width;
    const h = props.store.height;
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

    const solutionSet = new Set(solutionPath);
    const vt = viewTransform();

    if (topology === 'rectangular') {
      // Calculate cell size to fit canvas
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

      renderRectMaze(ctx, wallData, cellStates, solutionPath, solutionSet, w, h, start, end, cellSize, mazeW, mazeH);

      ctx.restore();
    } else if (topology === 'hexagonal') {
      const cellPositions = wasm.getCellPositions();
      if (cellPositions.length === 0) return;

      // Compute bounding box from cell positions
      const cellCount = wasm.getCellCount();
      let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
      for (let i = 0; i < cellCount; i++) {
        const x = cellPositions[i * 2];
        const y = cellPositions[i * 2 + 1];
        if (x < minX) minX = x;
        if (x > maxX) maxX = x;
        if (y < minY) minY = y;
        if (y > maxY) maxY = y;
      }

      const padding = 40;
      const bbW = maxX - minX;
      const bbH = maxY - minY;
      // Add margin for hex radius
      const marginFactor = 2.0;
      const scaleFactor = Math.max(2, Math.min(
        (displayWidth - 2 * padding) / (bbW + marginFactor),
        (displayHeight - 2 * padding) / (bbH + marginFactor),
      ));

      const mazePixW = (bbW + marginFactor) * scaleFactor;
      const mazePixH = (bbH + marginFactor) * scaleFactor;
      const offsetX = (displayWidth - mazePixW) / 2 - minX * scaleFactor + (marginFactor / 2) * scaleFactor;
      const offsetY = (displayHeight - mazePixH) / 2 - minY * scaleFactor + (marginFactor / 2) * scaleFactor;

      ctx.save();
      // Apply pan/zoom around center
      const cx = displayWidth / 2;
      const cy = displayHeight / 2;
      ctx.translate(cx + vt.offsetX, cy + vt.offsetY);
      ctx.scale(vt.scale, vt.scale);
      ctx.translate(-cx, -cy);

      renderHexMaze(ctx, wallData, cellStates, solutionPath, solutionSet, w, h, start, end, cellPositions, scaleFactor, offsetX, offsetY);

      ctx.restore();
    } else if (topology === 'triangular') {
      const cellPositions = wasm.getCellPositions();
      if (cellPositions.length === 0) return;

      const cellCount = wasm.getCellCount();
      let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
      for (let i = 0; i < cellCount; i++) {
        const x = cellPositions[i * 2];
        const y = cellPositions[i * 2 + 1];
        if (x < minX) minX = x;
        if (x > maxX) maxX = x;
        if (y < minY) minY = y;
        if (y > maxY) maxY = y;
      }

      const padding = 40;
      const bbW = maxX - minX;
      const bbH = maxY - minY;
      const marginFactor = 1.5;
      const scaleFactor = Math.max(2, Math.min(
        (displayWidth - 2 * padding) / (bbW + marginFactor),
        (displayHeight - 2 * padding) / (bbH + marginFactor),
      ));

      const mazePixW = (bbW + marginFactor) * scaleFactor;
      const mazePixH = (bbH + marginFactor) * scaleFactor;
      const offsetX = (displayWidth - mazePixW) / 2 - minX * scaleFactor + (marginFactor / 2) * scaleFactor;
      const offsetY = (displayHeight - mazePixH) / 2 - minY * scaleFactor + (marginFactor / 2) * scaleFactor;

      ctx.save();
      const cx = displayWidth / 2;
      const cy = displayHeight / 2;
      ctx.translate(cx + vt.offsetX, cy + vt.offsetY);
      ctx.scale(vt.scale, vt.scale);
      ctx.translate(-cx, -cy);

      renderTriMaze(ctx, wallData, cellStates, solutionPath, solutionSet, w, h, start, end, cellPositions, scaleFactor, offsetX, offsetY);

      ctx.restore();
    } else if (topology === 'circular') {
      const cellPositions = wasm.getCellPositions();
      const rings = w; // For circular, width = rings

      const padding = 40;
      const maxRadius = rings; // In grid units, outermost ring radius
      const scaleFactor = Math.max(2, Math.min(
        (displayWidth - 2 * padding) / (2 * maxRadius + 2),
        (displayHeight - 2 * padding) / (2 * maxRadius + 2),
      ));

      const centerX = displayWidth / 2;
      const centerY = displayHeight / 2;

      ctx.save();
      ctx.translate(centerX + vt.offsetX, centerY + vt.offsetY);
      ctx.scale(vt.scale, vt.scale);
      ctx.translate(-centerX, -centerY);

      renderCircularMaze(ctx, wallData, cellStates, solutionPath, solutionSet, start, end, cellPositions, scaleFactor, centerX, centerY, rings);

      ctx.restore();
    }
  };

  // Zoom handler (zoom toward cursor position)
  const handleWheel = (e: WheelEvent) => {
    e.preventDefault();
    const container = containerRef;
    if (!container) return;

    const rect = container.getBoundingClientRect();
    const mouseX = e.clientX - rect.left;
    const mouseY = e.clientY - rect.top;

    const vt = viewTransform();
    const zoomFactor = e.deltaY < 0 ? 1.1 : 1 / 1.1;
    const newScale = Math.max(0.2, Math.min(20, vt.scale * zoomFactor));

    const displayWidth = container.clientWidth;
    const displayHeight = container.clientHeight;

    const centerX = displayWidth / 2;
    const centerY = displayHeight / 2;

    const dx = mouseX - centerX - vt.offsetX;
    const dy = mouseY - centerY - vt.offsetY;

    const newOffsetX = vt.offsetX - dx * (zoomFactor - 1);
    const newOffsetY = vt.offsetY - dy * (zoomFactor - 1);

    setViewTransform({ offsetX: newOffsetX, offsetY: newOffsetY, scale: newScale });
  };

  // Pointer handlers -- tool-mode aware
  const handlePointerDown = (e: PointerEvent) => {
    if (e.button !== 0) return;

    const mode = props.store.toolMode();

    if (mode === 'pan') {
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
      const w = props.store.width;
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

    const w = props.store.width;
    const cellIndex = hit.row * w + hit.col;

    // Avoid re-toggling the same wall on drag
    if (lastWallAction && lastWallAction.cell === cellIndex && lastWallAction.dir === dir) return;

    lastWallAction = { cell: cellIndex, dir };
    applyWallEdit(cellIndex, dir, mode);
  };

  const handlePointerMove = (e: PointerEvent) => {
    const mode = props.store.toolMode();

    if (mode === 'pan' && isPanning) {
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

  /** Compute the CSS cursor based on tool mode. */
  const getCursor = (): string => {
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
    onCleanup(() => observer.disconnect());
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
    </div>
  );
};

export default MazeCanvas;
