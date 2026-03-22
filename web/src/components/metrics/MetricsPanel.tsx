import { Component } from 'solid-js';
import type { MazeState } from '../../stores/maze';
import { t } from '../../i18n';

function formatNumber(n: number): string {
  if (n >= 1000000) return (n / 1000000).toFixed(1) + 'M';
  if (n >= 1000) return n.toLocaleString();
  return String(n);
}

const MetricsPanel: Component<{ store: MazeState }> = (props) => {
  const cardBase: Record<string, string> = {
    padding: '10px 18px',
    background: 'var(--bg3)',
    'border-radius': '8px',
    border: '1px solid var(--border)',
    'min-width': '110px',
    transition: 'border-color 0.2s',
    cursor: 'default',
  };

  const labelStyle: Record<string, string> = {
    'font-size': '9px',
    'text-transform': 'uppercase',
    'letter-spacing': '0.1em',
    color: 'var(--text3)',
    'font-family': 'var(--font-mono)',
  };

  const valueBase: Record<string, string | number> = {
    'font-size': '22px',
    'font-weight': 700,
    'font-family': 'var(--font-mono)',
    'margin-top': '2px',
  };

  return (
    <div class="metrics-panel" style={{
      height: '90px',
      background: 'var(--bg2)',
      'border-top': '1px solid var(--border)',
      display: 'flex',
      'align-items': 'center',
      padding: '0 20px',
      gap: '14px',
      'flex-shrink': '0',
      'overflow-x': 'auto',
    }}>
      {/* Steps */}
      <div style={cardBase} class="metric-card">
        <div style={labelStyle}>{t('steps')}</div>
        <div style={{ ...valueBase, color: 'var(--cyan)' }}>
          {formatNumber(props.store.metrics().steps_taken)}
        </div>
      </div>

      {/* Visited */}
      <div style={cardBase} class="metric-card">
        <div style={labelStyle}>{t('visited')}</div>
        <div style={{ ...valueBase, color: 'var(--purple)' }}>
          {formatNumber(props.store.metrics().cells_visited)}
        </div>
      </div>

      {/* Path Length */}
      <div style={cardBase} class="metric-card">
        <div style={labelStyle}>{t('pathLength')}</div>
        <div style={{ ...valueBase, color: 'var(--green)' }}>
          {props.store.metrics().path_length || '\u2014'}
        </div>
      </div>

      {/* Dead Ends */}
      <div style={cardBase} class="metric-card">
        <div style={labelStyle}>{t('deadEnds')}</div>
        <div style={{ ...valueBase, color: 'var(--blue)' }}>
          {props.store.metrics().dead_ends || '\u2014'}
        </div>
      </div>

      {/* Efficiency */}
      <div style={cardBase} class="metric-card">
        <div style={labelStyle}>{t('efficiency')}</div>
        <div style={{ ...valueBase, color: 'var(--amber)' }}>
          {props.store.metrics().path_length > 0 && props.store.metrics().cells_visited > 0
            ? Math.round((props.store.metrics().path_length / props.store.metrics().cells_visited) * 100) + '%'
            : '\u2014'}
        </div>
      </div>

      {/* Maze size */}
      <div style={cardBase} class="metric-card">
        <div style={labelStyle}>{t('maze')}</div>
        <div style={{ ...valueBase, color: 'var(--text2)', 'font-size': '16px' }}>
          {props.store.width} x {props.store.height}
        </div>
      </div>
    </div>
  );
};

export default MetricsPanel;
