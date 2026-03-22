import { Component, createSignal, For, Show } from 'solid-js';
import type { CompareResult } from '../../stores/maze';

const SOLVER_COMPLEXITY: Record<string, string> = {
  bfs: 'O(V+E)', dfs: 'O(V+E)', astar: 'O(E\u00B7log V)', dijkstra: 'O(E\u00B7log V)',
  greedy_bfs: 'O(E\u00B7log V)', wall_follower: 'O(V)', tremaux: 'O(V+E)', dead_end_filling: 'O(V+E)',
};

type SortKey = 'algoName' | 'steps' | 'visited' | 'pathLength' | 'timeMs';
type SortDir = 'asc' | 'desc';

function formatNumber(n: number): string {
  if (n >= 1000000) return (n / 1000000).toFixed(1) + 'M';
  if (n >= 1000) return n.toLocaleString();
  return String(n);
}

const CompareResults: Component<{
  results: CompareResult[];
  onClose: () => void;
}> = (props) => {
  const [sortKey, setSortKey] = createSignal<SortKey>('pathLength');
  const [sortDir, setSortDir] = createSignal<SortDir>('asc');

  const handleHeaderClick = (key: SortKey) => {
    if (sortKey() === key) {
      setSortDir((prev) => (prev === 'asc' ? 'desc' : 'asc'));
    } else {
      setSortKey(key);
      setSortDir('asc');
    }
  };

  const sortedResults = () => {
    const key = sortKey();
    const dir = sortDir();
    return [...props.results].sort((a, b) => {
      let aVal: number | string;
      let bVal: number | string;

      if (key === 'algoName') {
        aVal = a.algoName;
        bVal = b.algoName;
      } else {
        aVal = a[key];
        bVal = b[key];
      }

      // For path length, push 0 (no path found) to the bottom
      if (key === 'pathLength') {
        if (aVal === 0 && bVal !== 0) return 1;
        if (bVal === 0 && aVal !== 0) return -1;
      }

      if (aVal < bVal) return dir === 'asc' ? -1 : 1;
      if (aVal > bVal) return dir === 'asc' ? 1 : -1;
      return 0;
    });
  };

  const sortArrow = (key: SortKey) => {
    if (sortKey() !== key) return '';
    return sortDir() === 'asc' ? ' \u25B2' : ' \u25BC';
  };

  const thStyle = (key: SortKey): Record<string, string> => ({
    padding: '8px 12px',
    'text-align': key === 'algoName' ? 'left' : 'right',
    'font-size': '9px',
    'text-transform': 'uppercase',
    'letter-spacing': '0.1em',
    color: sortKey() === key ? 'var(--cyan)' : 'var(--text3)',
    'font-family': 'var(--font-mono)',
    'font-weight': '600',
    cursor: 'pointer',
    'white-space': 'nowrap',
    'user-select': 'none',
    'border-bottom': '1px solid var(--border)',
    transition: 'color 0.15s',
  });

  return (
    <div style={{
      position: 'absolute',
      inset: '0',
      background: 'rgba(6,8,13,0.85)',
      'backdrop-filter': 'blur(8px)',
      display: 'flex',
      'align-items': 'center',
      'justify-content': 'center',
      'z-index': '100',
    }}>
      <div style={{
        background: 'var(--bg2)',
        border: '1px solid var(--border)',
        'border-radius': '12px',
        'max-width': '820px',
        width: '90%',
        'max-height': '80%',
        overflow: 'hidden',
        display: 'flex',
        'flex-direction': 'column',
        'box-shadow': '0 24px 80px rgba(0,0,0,0.5)',
      }}>
        {/* Header */}
        <div style={{
          display: 'flex',
          'align-items': 'center',
          'justify-content': 'space-between',
          padding: '16px 20px',
          'border-bottom': '1px solid var(--border)',
        }}>
          <div style={{
            'font-family': 'var(--font-mono)',
            'font-size': '13px',
            'font-weight': 700,
            color: 'var(--cyan)',
          }}>
            Auto Compare Results
          </div>
          <button
            onClick={props.onClose}
            style={{
              width: '28px',
              height: '28px',
              'border-radius': '6px',
              display: 'flex',
              'align-items': 'center',
              'justify-content': 'center',
              background: 'var(--bg3)',
              border: '1px solid var(--border)',
              color: 'var(--text2)',
              'font-size': '14px',
              cursor: 'pointer',
              transition: 'all 0.15s',
            }}
          >
            {'\u2715'}
          </button>
        </div>

        {/* Table */}
        <div style={{ 'overflow-y': 'auto', padding: '0' }}>
          <table style={{
            width: '100%',
            'border-collapse': 'collapse',
            'font-size': '13px',
          }}>
            <thead>
              <tr>
                <th style={thStyle('algoName')} onClick={() => handleHeaderClick('algoName')}>
                  Algorithm{sortArrow('algoName')}
                </th>
                <th style={thStyle('steps')} onClick={() => handleHeaderClick('steps')}>
                  Steps{sortArrow('steps')}
                </th>
                <th style={thStyle('visited')} onClick={() => handleHeaderClick('visited')}>
                  Visited{sortArrow('visited')}
                </th>
                <th style={thStyle('pathLength')} onClick={() => handleHeaderClick('pathLength')}>
                  Path{sortArrow('pathLength')}
                </th>
                <th style={thStyle('timeMs')} onClick={() => handleHeaderClick('timeMs')}>
                  Time{sortArrow('timeMs')}
                </th>
                <th style={{
                  padding: '8px 12px',
                  'text-align': 'right',
                  'font-size': '9px',
                  'text-transform': 'uppercase',
                  'letter-spacing': '0.1em',
                  color: 'var(--text3)',
                  'font-family': 'var(--font-mono)',
                  'font-weight': '600',
                  'white-space': 'nowrap',
                  'user-select': 'none',
                  'border-bottom': '1px solid var(--border)',
                }}>
                  Complexity
                </th>
              </tr>
            </thead>
            <tbody>
              <For each={sortedResults()}>
                {(row) => (
                  <tr style={{
                    'border-left': row.isBest ? '2px solid var(--cyan)' : '2px solid transparent',
                    background: row.isBest ? 'rgba(0,212,170,0.04)' : 'transparent',
                    transition: 'background 0.15s',
                  }}>
                    <td style={{
                      padding: '10px 12px',
                      'font-family': 'var(--font-sans)',
                      color: row.isBest ? 'var(--cyan)' : 'var(--text)',
                      'font-weight': row.isBest ? 600 : 400,
                      'border-bottom': '1px solid rgba(26,39,68,0.3)',
                    }}>
                      {row.algoName}
                      <Show when={row.isBest}>
                        <span style={{
                          'margin-left': '8px',
                          'font-size': '9px',
                          'text-transform': 'uppercase',
                          'letter-spacing': '0.08em',
                          color: 'var(--cyan)',
                          background: 'rgba(0,212,170,0.12)',
                          padding: '1px 6px',
                          'border-radius': '3px',
                        }}>
                          best
                        </span>
                      </Show>
                    </td>
                    <td style={{
                      padding: '10px 12px',
                      'text-align': 'right',
                      'font-family': 'var(--font-mono)',
                      color: 'var(--text)',
                      'border-bottom': '1px solid rgba(26,39,68,0.3)',
                    }}>
                      {formatNumber(row.steps)}
                    </td>
                    <td style={{
                      padding: '10px 12px',
                      'text-align': 'right',
                      'font-family': 'var(--font-mono)',
                      color: 'var(--purple)',
                      'border-bottom': '1px solid rgba(26,39,68,0.3)',
                    }}>
                      {formatNumber(row.visited)}
                    </td>
                    <td style={{
                      padding: '10px 12px',
                      'text-align': 'right',
                      'font-family': 'var(--font-mono)',
                      color: row.pathLength > 0 ? 'var(--green)' : 'var(--text3)',
                      'font-weight': 700,
                      'border-bottom': '1px solid rgba(26,39,68,0.3)',
                    }}>
                      {row.pathLength > 0 ? formatNumber(row.pathLength) : '\u2014'}
                    </td>
                    <td style={{
                      padding: '10px 12px',
                      'text-align': 'right',
                      'font-family': 'var(--font-mono)',
                      color: 'var(--text2)',
                      'border-bottom': '1px solid rgba(26,39,68,0.3)',
                    }}>
                      {row.timeMs < 1 ? '<1' : row.timeMs.toFixed(1)}ms
                    </td>
                    <td style={{
                      padding: '10px 12px',
                      'text-align': 'right',
                      'font-family': 'var(--font-mono)',
                      color: 'var(--text2)',
                      'border-bottom': '1px solid rgba(26,39,68,0.3)',
                    }}>
                      {SOLVER_COMPLEXITY[row.algo] || '\u2014'}
                    </td>
                  </tr>
                )}
              </For>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};

export default CompareResults;
