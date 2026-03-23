import { Component, For, createSignal } from 'solid-js';
import type { RunHistoryEntry } from '../../stores/maze';
import { t } from '../../i18n';

interface Props {
  history: () => RunHistoryEntry[];
  onReRun: (entry: RunHistoryEntry) => void;
}

const RunHistory: Component<Props> = (props) => {
  const [expanded, setExpanded] = createSignal(false);

  const headerStyle: Record<string, string> = {
    display: 'flex',
    'align-items': 'center',
    'justify-content': 'space-between',
    padding: '8px 20px',
    cursor: 'pointer',
    'user-select': 'none',
    background: 'var(--bg2)',
    'border-top': '1px solid var(--border)',
    'font-size': '12px',
    'font-family': 'var(--font-mono)',
    color: 'var(--text2)',
    'flex-shrink': '0',
  };

  const cellStyle: Record<string, string> = {
    padding: '4px 8px',
    'font-size': '11px',
    'font-family': 'var(--font-mono)',
    color: 'var(--text)',
    'white-space': 'nowrap',
    'text-align': 'right',
  };

  const thStyle: Record<string, string> = {
    ...cellStyle,
    color: 'var(--text3)',
    'font-size': '9px',
    'text-transform': 'uppercase',
    'letter-spacing': '0.08em',
    'border-bottom': '1px solid var(--border)',
    position: 'sticky',
    top: '0',
    background: 'var(--bg2)',
  };

  return (
    <div style={{ 'flex-shrink': '0' }}>
      <div style={headerStyle} onClick={() => setExpanded((v) => !v)}>
        <span>
          {t('runHistory')} ({props.history().length})
        </span>
        <span style={{
          transform: expanded() ? 'rotate(180deg)' : 'rotate(0deg)',
          transition: 'transform 0.2s',
          'font-size': '10px',
        }}>
          {'\u25BC'}
        </span>
      </div>
      {expanded() && props.history().length > 0 && (
        <div style={{
          background: 'var(--bg2)',
          'border-top': '1px solid var(--border)',
          'max-height': '150px',
          'overflow-y': 'auto',
          'overflow-x': 'auto',
          'flex-shrink': '0',
        }}>
          <table style={{
            width: '100%',
            'border-collapse': 'collapse',
            'min-width': '600px',
          }}>
            <thead>
              <tr>
                <th style={{ ...thStyle, 'text-align': 'center' }}>#</th>
                <th style={{ ...thStyle, 'text-align': 'left' }}>Algorithm</th>
                <th style={thStyle}>Size</th>
                <th style={thStyle}>Seed</th>
                <th style={thStyle}>Steps</th>
                <th style={thStyle}>Visited</th>
                <th style={thStyle}>Path</th>
                <th style={thStyle}>Time</th>
                <th style={{ ...thStyle, 'text-align': 'center' }}></th>
              </tr>
            </thead>
            <tbody>
              <For each={props.history()}>
                {(entry, i) => (
                  <tr style={{
                    background: i() % 2 === 0 ? 'var(--bg)' : 'var(--bg2)',
                    transition: 'background 0.1s',
                  }}>
                    <td style={{ ...cellStyle, 'text-align': 'center', color: 'var(--text3)' }}>
                      {entry.id}
                    </td>
                    <td style={{ ...cellStyle, 'text-align': 'left', color: 'var(--cyan)' }}>
                      {entry.algoName}
                    </td>
                    <td style={cellStyle}>
                      {entry.width}x{entry.height}
                    </td>
                    <td style={cellStyle}>
                      {entry.seed}
                    </td>
                    <td style={cellStyle}>
                      {entry.steps.toLocaleString()}
                    </td>
                    <td style={{ ...cellStyle, color: 'var(--purple)' }}>
                      {entry.visited.toLocaleString()}
                    </td>
                    <td style={{ ...cellStyle, color: 'var(--green)' }}>
                      {entry.pathLength || '\u2014'}
                    </td>
                    <td style={cellStyle}>
                      {entry.timeMs.toFixed(1)}ms
                    </td>
                    <td style={{ ...cellStyle, 'text-align': 'center' }}>
                      <button
                        title={t('reRun')}
                        onClick={() => props.onReRun(entry)}
                        style={{
                          width: '24px',
                          height: '24px',
                          'border-radius': '4px',
                          'font-size': '11px',
                          display: 'inline-flex',
                          'align-items': 'center',
                          'justify-content': 'center',
                          cursor: 'pointer',
                          background: 'rgba(0,212,170,0.08)',
                          border: '1px solid var(--border)',
                          color: 'var(--cyan)',
                          transition: 'all 0.15s',
                        }}
                      >
                        {'\u25B6'}
                      </button>
                    </td>
                  </tr>
                )}
              </For>
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};

export default RunHistory;
