import { Component, createEffect, onMount } from 'solid-js';
import type { GraphDataPoint } from '../../stores/maze';

interface Props {
  data: () => GraphDataPoint[];
  color: string;
  label: string;
  height?: number;
}

const RealtimeGraph: Component<Props> = (props) => {
  let canvasRef: HTMLCanvasElement | undefined;

  const graphHeight = () => props.height ?? 40;

  const render = () => {
    const canvas = canvasRef;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    // Background
    ctx.fillStyle = 'var(--bg3)';
    ctx.fillRect(0, 0, w, h);
    // Fallback: parse computed style for bg3
    const computedBg = getComputedStyle(canvas).getPropertyValue('--bg3') || '#161b22';
    ctx.fillStyle = computedBg.trim();
    ctx.fillRect(0, 0, w, h);

    const data = props.data();
    if (data.length < 2) {
      // Draw label even with no data
      ctx.fillStyle = props.color;
      ctx.font = '10px "JetBrains Mono", monospace';
      ctx.textAlign = 'left';
      ctx.textBaseline = 'top';
      ctx.fillText(props.label, 4, 3);
      return;
    }

    const minStep = data[0].step;
    const maxStep = data[data.length - 1].step;
    const stepRange = maxStep - minStep || 1;

    let maxVal = 0;
    for (const pt of data) {
      if (pt.value > maxVal) maxVal = pt.value;
    }
    if (maxVal === 0) maxVal = 1;

    const pad = 2;
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

    // Parse color for alpha fill
    ctx.fillStyle = props.color;
    ctx.globalAlpha = 0.15;
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
    ctx.strokeStyle = props.color;
    ctx.lineWidth = 1.5;
    ctx.stroke();

    // Label (top-left)
    ctx.fillStyle = props.color;
    ctx.font = '10px "JetBrains Mono", monospace';
    ctx.textAlign = 'left';
    ctx.textBaseline = 'top';
    ctx.fillText(props.label, 4, 3);

    // Latest value (top-right)
    const latest = data[data.length - 1].value;
    const valText = latest >= 1000 ? (latest / 1000).toFixed(1) + 'k' : String(latest);
    ctx.textAlign = 'right';
    ctx.fillText(valText, w - 4, 3);
  };

  onMount(() => {
    render();
  });

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
        'min-width': '80px',
      }}
    />
  );
};

export default RealtimeGraph;
