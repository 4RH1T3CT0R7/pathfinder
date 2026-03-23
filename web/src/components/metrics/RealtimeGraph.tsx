import { Component, createEffect, onMount } from 'solid-js';
import type { GraphDataPoint } from '../../stores/maze';

interface Props {
  data: () => GraphDataPoint[];
  color: string;
  label: string;
  height?: number;
}

// Resolve CSS variable to actual hex color
function resolveColor(color: string, el?: HTMLElement): string {
  if (!color.startsWith('var(')) return color;
  const varName = color.replace(/var\(([^)]+)\)/, '$1').trim();
  const fallbacks: Record<string, string> = {
    '--blue': '#58a6ff',
    '--purple': '#7b61ff',
    '--cyan': '#00d4aa',
    '--amber': '#f0b429',
    '--red': '#ff6b6b',
    '--green': '#34d399',
  };
  if (el) {
    const val = getComputedStyle(el).getPropertyValue(varName).trim();
    if (val) return val;
  }
  return fallbacks[varName] || '#58a6ff';
}

const RealtimeGraph: Component<Props> = (props) => {
  let canvasRef: HTMLCanvasElement | undefined;

  const graphHeight = () => props.height ?? 48;

  const render = () => {
    const canvas = canvasRef;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    if (w === 0 || h === 0) return;
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    const accentColor = resolveColor(props.color, canvas);

    // Background
    ctx.fillStyle = '#0d1117';
    ctx.fillRect(0, 0, w, h);

    // Grid lines (subtle)
    ctx.strokeStyle = '#1a2744';
    ctx.lineWidth = 0.5;
    for (let i = 1; i < 4; i++) {
      const gy = (h / 4) * i;
      ctx.beginPath();
      ctx.moveTo(0, gy);
      ctx.lineTo(w, gy);
      ctx.stroke();
    }

    const data = props.data();

    // Label (top-left) — always show
    ctx.fillStyle = accentColor;
    ctx.font = 'bold 10px "JetBrains Mono", monospace';
    ctx.textAlign = 'left';
    ctx.textBaseline = 'top';
    ctx.fillText(props.label, 6, 4);

    if (data.length < 2) return;

    const minStep = data[0].step;
    const maxStep = data[data.length - 1].step;
    const stepRange = maxStep - minStep || 1;

    let maxVal = 0;
    for (const pt of data) {
      if (pt.value > maxVal) maxVal = pt.value;
    }
    if (maxVal === 0) maxVal = 1;

    const pad = 4;
    const plotW = w - pad * 2;
    const plotH = h - pad * 2;

    const toX = (step: number) => pad + ((step - minStep) / stepRange) * plotW;
    const toY = (value: number) => pad + plotH - (value / maxVal) * plotH;

    // Filled area under curve
    ctx.beginPath();
    ctx.moveTo(toX(data[0].step), toY(0));
    for (const pt of data) {
      ctx.lineTo(toX(pt.step), toY(pt.value));
    }
    ctx.lineTo(toX(data[data.length - 1].step), toY(0));
    ctx.closePath();
    ctx.fillStyle = accentColor;
    ctx.globalAlpha = 0.2;
    ctx.fill();
    ctx.globalAlpha = 1;

    // Line
    ctx.beginPath();
    for (let i = 0; i < data.length; i++) {
      const x = toX(data[i].step);
      const y = toY(data[i].value);
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.strokeStyle = accentColor;
    ctx.lineWidth = 2;
    ctx.stroke();

    // Latest value (top-right)
    const latest = data[data.length - 1].value;
    const valText = latest >= 1000 ? (latest / 1000).toFixed(1) + 'k' : String(latest);
    ctx.fillStyle = accentColor;
    ctx.font = 'bold 10px "JetBrains Mono", monospace';
    ctx.textAlign = 'right';
    ctx.textBaseline = 'top';
    ctx.fillText(valText, w - 6, 4);

    // Max value label (right side, vertically centered)
    const maxText = maxVal >= 1000 ? (maxVal / 1000).toFixed(1) + 'k' : String(maxVal);
    ctx.fillStyle = '#484f58';
    ctx.font = '9px "JetBrains Mono", monospace';
    ctx.textAlign = 'right';
    ctx.textBaseline = 'top';
    ctx.fillText('max:' + maxText, w - 6, 16);
  };

  onMount(() => render());

  createEffect(() => {
    props.data();
    render();
  });

  return (
    <canvas
      ref={canvasRef}
      style={{
        flex: '1',
        height: `${graphHeight()}px`,
        'border-radius': '6px',
        border: '1px solid var(--border)',
        display: 'block',
        'min-width': '100px',
      }}
    />
  );
};

export default RealtimeGraph;
