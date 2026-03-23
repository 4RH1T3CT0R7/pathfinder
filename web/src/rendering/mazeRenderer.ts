// =============================================================================
// Shared maze rendering module
// =============================================================================
// Extracted from MazeCanvas.tsx so that both MazeCanvas and ComparisonView
// can render all topologies (rectangular, hexagonal, triangular, circular).
// =============================================================================

// Wall direction bitmasks (rectangular)
export const WALL_N = 0b0001;
export const WALL_E = 0b0010;
export const WALL_S = 0b0100;
export const WALL_W = 0b1000;

// Cell state constants
const CELL_UNVISITED = 0;
const CELL_FRONTIER = 1;
const CELL_VISITED = 2;
const CELL_ACTIVE = 3;
const CELL_SOLUTION = 4;
const CELL_BACKTRACKED = 5;

// Cell state colors
export const CELL_COLORS: Record<number, string> = {
  [CELL_UNVISITED]: '#0a0e14',
  [CELL_FRONTIER]: 'rgba(88,166,255,0.4)',
  [CELL_VISITED]: 'rgba(123,97,255,0.3)',
  [CELL_ACTIVE]: 'rgba(0,212,170,0.55)',
  [CELL_SOLUTION]: 'rgba(52,211,153,0.75)',
  [CELL_BACKTRACKED]: 'rgba(255,75,75,0.2)',
};

// ---- Data interfaces -------------------------------------------------------

export interface MazeRenderData {
  wallData: Uint8Array;
  cellStates: Uint8Array | null;
  solutionPath: number[];
  w: number;
  h: number;
  startCell: number;
  endCell: number;
  topology: string; // 'rectangular' | 'hexagonal' | 'triangular' | 'circular'
  cellPositions?: Float64Array; // for non-rect topologies
}

// ---- Helpers ---------------------------------------------------------------

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

/** Compute the 6 vertices of a pointy-top hexagon centered at (cx, cy) with size s. */
export function hexVertices(cx: number, cy: number, s: number): [number, number][] {
  const verts: [number, number][] = [];
  for (let i = 0; i < 6; i++) {
    const angle = Math.PI / 6 + i * Math.PI / 3;
    verts.push([cx + s * Math.cos(angle), cy + s * Math.sin(angle)]);
  }
  return verts;
}

/** Get the 3 vertices of a triangle cell. */
export function triVertices(
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

// ---- Topology-specific renderers -------------------------------------------

function renderRectMaze(
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
): void {
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
}

function renderHexMaze(
  ctx: CanvasRenderingContext2D,
  wallData: Uint8Array,
  cellStates: Uint8Array | null,
  solutionPath: number[],
  solutionSet: Set<number>,
  start: number,
  end: number,
  cellPositions: Float64Array,
  cellCount: number,
  scaleFactor: number,
  offsetX: number,
  offsetY: number,
): void {
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

  // Draw walls: each hex cell has 6 edges.
  // Edge i connects vertex i to vertex (i+1)%6 in our vertex array.
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
}

function renderTriMaze(
  ctx: CanvasRenderingContext2D,
  wallData: Uint8Array,
  cellStates: Uint8Array | null,
  solutionPath: number[],
  solutionSet: Set<number>,
  w: number,
  start: number,
  end: number,
  cellPositions: Float64Array,
  cellCount: number,
  scaleFactor: number,
  offsetX: number,
  offsetY: number,
): void {
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

    // LEFT (bit 0)
    if (walls & 0b001) {
      wallPath.moveTo(verts[0][0], verts[0][1]);
      wallPath.lineTo(verts[2][0], verts[2][1]);
    }
    // RIGHT (bit 1)
    if (walls & 0b010) {
      wallPath.moveTo(verts[2][0], verts[2][1]);
      wallPath.lineTo(verts[1][0], verts[1][1]);
    }
    // BASE (bit 2)
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
}

function renderCircularMaze(
  ctx: CanvasRenderingContext2D,
  wallData: Uint8Array,
  cellStates: Uint8Array | null,
  solutionPath: number[],
  solutionSet: Set<number>,
  start: number,
  end: number,
  cellPositions: Float64Array,
  cellCount: number,
  scaleFactor: number,
  centerX: number,
  centerY: number,
  rings: number,
): void {
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
  // Circular wall directions: INWARD(0) CW(1) OUTWARD(2) CCW(3)
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
      // Center cell: outward walls are drawn as arcs at each 60-degree sector
      const outerR = ringWidth;
      for (let d = 0; d < 6; d++) {
        if (walls & (1 << d)) {
          const angle = (d / 6) * Math.PI * 2 - Math.PI / 2;
          const nextAngle = ((d + 1) / 6) * Math.PI * 2 - Math.PI / 2;
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
}

// ---- Bounding box helper for position-based topologies ---------------------

function computeBounds(cellPositions: Float64Array, cellCount: number): {
  minX: number; maxX: number; minY: number; maxY: number;
} {
  let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
  for (let i = 0; i < cellCount; i++) {
    const x = cellPositions[i * 2];
    const y = cellPositions[i * 2 + 1];
    if (x < minX) minX = x;
    if (x > maxX) maxX = x;
    if (y < minY) minY = y;
    if (y > maxY) maxY = y;
  }
  return { minX, maxX, minY, maxY };
}

// ---- Main entry point ------------------------------------------------------

/**
 * Render a maze of any topology onto a canvas 2D context.
 *
 * The caller is responsible for:
 * - Setting up DPR scaling on the context
 * - Clearing the canvas background
 * - Applying any zoom/pan transforms (ctx.save/translate/scale before, restore after)
 *
 * This function draws the maze content (cells, walls, solution, border) into
 * the coordinate space of [0, 0] .. [displayWidth, displayHeight].
 * Internally it centers the maze and fits it into the available area.
 */
export function renderMazeToCanvas(
  ctx: CanvasRenderingContext2D,
  data: MazeRenderData,
  displayWidth: number,
  displayHeight: number,
): void {
  const { wallData, cellStates, solutionPath, w, h, startCell, endCell, topology, cellPositions } = data;

  if (!wallData || wallData.length === 0) return;

  const solutionSet = new Set(solutionPath);
  const cellCount = wallData.length;

  if (topology === 'rectangular') {
    const padding = 4;
    const availW = displayWidth - 2 * padding;
    const availH = displayHeight - 2 * padding;
    const cellSize = Math.max(1, Math.min(availW / w, availH / h));
    const mazeW = cellSize * w;
    const mazeH = cellSize * h;
    const baseOffsetX = (displayWidth - mazeW) / 2;
    const baseOffsetY = (displayHeight - mazeH) / 2;

    ctx.save();
    ctx.translate(baseOffsetX, baseOffsetY);
    renderRectMaze(ctx, wallData, cellStates, solutionPath, solutionSet, w, h, startCell, endCell, cellSize, mazeW, mazeH);
    ctx.restore();
  } else if (topology === 'hexagonal') {
    if (!cellPositions || cellPositions.length === 0) return;

    const bounds = computeBounds(cellPositions, cellCount);
    const padding = 20;
    const bbW = bounds.maxX - bounds.minX;
    const bbH = bounds.maxY - bounds.minY;
    const marginFactor = 2.0;
    const scaleFactor = Math.max(2, Math.min(
      (displayWidth - 2 * padding) / (bbW + marginFactor),
      (displayHeight - 2 * padding) / (bbH + marginFactor),
    ));

    const mazePixW = (bbW + marginFactor) * scaleFactor;
    const mazePixH = (bbH + marginFactor) * scaleFactor;
    const offsetX = (displayWidth - mazePixW) / 2 - bounds.minX * scaleFactor + (marginFactor / 2) * scaleFactor;
    const offsetY = (displayHeight - mazePixH) / 2 - bounds.minY * scaleFactor + (marginFactor / 2) * scaleFactor;

    renderHexMaze(ctx, wallData, cellStates, solutionPath, solutionSet, startCell, endCell, cellPositions, cellCount, scaleFactor, offsetX, offsetY);
  } else if (topology === 'triangular') {
    if (!cellPositions || cellPositions.length === 0) return;

    const bounds = computeBounds(cellPositions, cellCount);
    const padding = 20;
    const bbW = bounds.maxX - bounds.minX;
    const bbH = bounds.maxY - bounds.minY;
    const marginFactor = 1.5;
    const scaleFactor = Math.max(2, Math.min(
      (displayWidth - 2 * padding) / (bbW + marginFactor),
      (displayHeight - 2 * padding) / (bbH + marginFactor),
    ));

    const mazePixW = (bbW + marginFactor) * scaleFactor;
    const mazePixH = (bbH + marginFactor) * scaleFactor;
    const offsetX = (displayWidth - mazePixW) / 2 - bounds.minX * scaleFactor + (marginFactor / 2) * scaleFactor;
    const offsetY = (displayHeight - mazePixH) / 2 - bounds.minY * scaleFactor + (marginFactor / 2) * scaleFactor;

    renderTriMaze(ctx, wallData, cellStates, solutionPath, solutionSet, w, startCell, endCell, cellPositions, cellCount, scaleFactor, offsetX, offsetY);
  } else if (topology === 'circular') {
    if (!cellPositions || cellPositions.length === 0) return;

    const rings = w; // For circular, width = rings
    const padding = 20;
    const maxRadius = rings;
    const scaleFactor = Math.max(2, Math.min(
      (displayWidth - 2 * padding) / (2 * maxRadius + 2),
      (displayHeight - 2 * padding) / (2 * maxRadius + 2),
    ));

    const centerX = displayWidth / 2;
    const centerY = displayHeight / 2;

    renderCircularMaze(ctx, wallData, cellStates, solutionPath, solutionSet, startCell, endCell, cellPositions, cellCount, scaleFactor, centerX, centerY, rings);
  }
}
