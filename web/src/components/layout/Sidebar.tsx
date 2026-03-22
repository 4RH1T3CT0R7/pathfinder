import { Component, Show } from 'solid-js';
import type { MazeState, SolverAlgo, Topology, ToolMode } from '../../stores/maze';
import { RECT_ONLY_GENERATORS } from '../../stores/maze';
import { t } from '../../i18n';
import PresetPicker from '../controls/PresetPicker';
import ExportMenu from '../export/ExportMenu';

const sectionStyle = {
  padding: '14px 16px',
  'border-bottom': '1px solid rgba(26,39,68,0.5)',
};

const labelStyle = {
  'font-size': '10px',
  'text-transform': 'uppercase' as const,
  'letter-spacing': '0.12em',
  color: 'var(--text3)',
  'font-family': 'var(--font-mono)',
  'margin-bottom': '10px',
};

const Sidebar: Component<{
  store: MazeState;
  onGenerate: () => void;
  onSolve: () => void;
  onReset: () => void;
  onAutoCompare?: () => void;
  onRecordVideo?: () => void;
}> = (props) => {
  return (
    <aside class="sidebar">
      {/* Topology */}
      <div style={sectionStyle}>
        <div style={labelStyle}>{t('topology')}</div>
        <select
          value={props.store.topology()}
          onChange={(e) => props.store.setTopology(e.currentTarget.value as Topology)}
          style={{ width: '100%' }}
        >
          <option value="rectangular">{t('rectangular')}</option>
          <option value="hexagonal">{t('hexagonal')}</option>
          <option value="triangular">{t('triangular')}</option>
          <option value="circular">{t('circular')}</option>
        </select>
      </div>

      {/* Maze Size */}
      <div style={sectionStyle}>
        <div style={labelStyle}>{t('size')}</div>
        <Show
          when={props.store.topology() !== 'circular'}
          fallback={
            <div>
              <label style={{ 'font-size': '11px', color: 'var(--text2)', display: 'block', 'margin-bottom': '4px' }}>{t('rings')}</label>
              <input
                type="number"
                min={1}
                max={100}
                value={props.store.width}
                onInput={(e) => props.store.setWidth(parseInt(e.currentTarget.value) || 5)}
                style={{ width: '100%' }}
              />
            </div>
          }
        >
          <div style={{ display: 'flex', gap: '8px' }}>
            <div style={{ flex: '1' }}>
              <label style={{ 'font-size': '11px', color: 'var(--text2)', display: 'block', 'margin-bottom': '4px' }}>{t('width')}</label>
              <input
                type="number"
                min={3}
                max={1000}
                value={props.store.width}
                onInput={(e) => props.store.setWidth(parseInt(e.currentTarget.value) || 20)}
                style={{ width: '100%' }}
              />
            </div>
            <div style={{ flex: '1' }}>
              <label style={{ 'font-size': '11px', color: 'var(--text2)', display: 'block', 'margin-bottom': '4px' }}>{t('height')}</label>
              <input
                type="number"
                min={3}
                max={1000}
                value={props.store.height}
                onInput={(e) => props.store.setHeight(parseInt(e.currentTarget.value) || 20)}
                style={{ width: '100%' }}
              />
            </div>
          </div>
        </Show>
      </div>

      {/* Seed */}
      <div style={sectionStyle}>
        <div style={labelStyle}>{t('seed')}</div>
        <div style={{ display: 'flex', gap: '8px' }}>
          <input
            type="number"
            value={props.store.seed}
            onInput={(e) => props.store.setSeed(parseInt(e.currentTarget.value) || 0)}
            style={{ flex: '1' }}
          />
          <button
            onClick={() => props.store.setSeed(Math.floor(Math.random() * 999999))}
            style={{
              padding: '7px 14px',
              background: 'var(--bg3)',
              border: '1px solid var(--border)',
              'border-radius': '6px',
              color: 'var(--text2)',
              'font-size': '12px',
              cursor: 'pointer',
              transition: 'all 0.15s',
            }}
          >
            Rnd
          </button>
        </div>
      </div>

      {/* Presets */}
      <PresetPicker store={props.store} onApply={props.onGenerate} />

      {/* Generation Algorithm */}
      <div style={sectionStyle}>
        <div style={labelStyle}>{t('generation')}</div>
        <select
          value={props.store.generatorAlgo()}
          onChange={(e) => props.store.setGeneratorAlgo(e.currentTarget.value as any)}
          style={{ width: '100%' }}
        >
          {(() => {
            const isRect = props.store.topology() === 'rectangular';
            const generators = [
              { value: 'dfs', label: 'DFS Recursive Backtracker' },
              { value: 'kruskal', label: 'Randomized Kruskal' },
              { value: 'prim', label: 'Randomized Prim' },
              { value: 'eller', label: "Eller's Algorithm" },
              { value: 'wilson', label: "Wilson's Algorithm" },
              { value: 'growing_tree', label: 'Growing Tree' },
              { value: 'binary_tree', label: 'Binary Tree' },
              { value: 'sidewinder', label: 'Sidewinder' },
              { value: 'aldous_broder', label: 'Aldous-Broder' },
              { value: 'hunt_and_kill', label: 'Hunt and Kill' },
            ];
            return generators
              .filter((g) => isRect || !RECT_ONLY_GENERATORS.includes(g.value as any))
              .map((g) => <option value={g.value}>{g.label}</option>);
          })()}
        </select>
      </div>

      {/* Solving Algorithm */}
      <div style={sectionStyle}>
        <div style={labelStyle}>{t('solving')}</div>
        <select
          value={props.store.solverAlgo()}
          onChange={(e) => props.store.setSolverAlgo(e.currentTarget.value as SolverAlgo)}
          style={{ width: '100%' }}
        >
          <option value="bfs">BFS (Breadth-First)</option>
          <option value="dfs">DFS (Depth-First)</option>
          <option value="astar">A* Search</option>
          <option value="dijkstra">Dijkstra</option>
          <option value="greedy_bfs">Greedy Best-First</option>
          <option value="wall_follower">Wall Follower</option>
          <option value="tremaux">Tremaux</option>
          <option value="dead_end_filling">Dead-End Filling</option>
        </select>

        {/* Compare mode toggle */}
        <div style={{ 'margin-top': '10px' }}>
          <label style={{
            display: 'flex',
            'align-items': 'center',
            gap: '8px',
            cursor: 'pointer',
            'font-size': '12px',
            color: props.store.compareMode() ? 'var(--cyan)' : 'var(--text2)',
            transition: 'color 0.15s',
            'user-select': 'none',
          }}>
            <div
              onClick={() => props.store.setCompareMode(!props.store.compareMode())}
              style={{
                width: '32px',
                height: '18px',
                'border-radius': '9px',
                background: props.store.compareMode() ? 'var(--cyan)' : 'var(--bg3)',
                border: `1px solid ${props.store.compareMode() ? 'var(--cyan)' : 'var(--border)'}`,
                position: 'relative',
                transition: 'all 0.2s',
                cursor: 'pointer',
                'flex-shrink': '0',
              }}
            >
              <div style={{
                width: '12px',
                height: '12px',
                'border-radius': '50%',
                background: props.store.compareMode() ? 'var(--bg)' : 'var(--text3)',
                position: 'absolute',
                top: '2px',
                left: props.store.compareMode() ? '16px' : '2px',
                transition: 'all 0.2s',
              }} />
            </div>
            <span onClick={() => props.store.setCompareMode(!props.store.compareMode())}>
              {t('compareMode')}
            </span>
          </label>
        </div>

        {/* Second algorithm selector (shown when compare mode is on) */}
        <Show when={props.store.compareMode()}>
          <div style={{ 'margin-top': '8px' }}>
            <label style={{ 'font-size': '11px', color: 'var(--text2)', display: 'block', 'margin-bottom': '4px' }}>
              {t('compareWith')}
            </label>
            <select
              value={props.store.compareAlgo()}
              onChange={(e) => props.store.setCompareAlgo(e.currentTarget.value as SolverAlgo)}
              style={{ width: '100%' }}
            >
              <option value="bfs">BFS (Breadth-First)</option>
              <option value="dfs">DFS (Depth-First)</option>
              <option value="astar">A* Search</option>
              <option value="dijkstra">Dijkstra</option>
              <option value="greedy_bfs">Greedy Best-First</option>
              <option value="wall_follower">Wall Follower</option>
              <option value="tremaux">Tremaux</option>
              <option value="dead_end_filling">Dead-End Filling</option>
            </select>
          </div>
        </Show>
      </div>

      {/* Speed */}
      <div style={sectionStyle}>
        <div style={labelStyle}>{t('speed')}</div>
        <input
          type="range"
          min={0}
          max={100}
          value={props.store.speed()}
          onInput={(e) => props.store.setSpeed(parseInt(e.currentTarget.value))}
          style={{ width: '100%', margin: '4px 0' }}
        />
        <div style={{
          display: 'flex',
          'justify-content': 'space-between',
          'font-size': '10px',
          color: 'var(--text3)',
          'font-family': 'var(--font-mono)',
        }}>
          <span>{t('slow')}</span>
          <span>{props.store.speed()}%</span>
          <span>{t('fast')}</span>
        </div>
      </div>

      {/* Tools */}
      <div style={sectionStyle}>
        <div style={labelStyle}>{t('tools')}</div>
        <div style={{ display: 'flex', gap: '4px', 'flex-wrap': 'wrap' }}>
          {([
            { mode: 'pan' as ToolMode, label: '\u270B', title: 'Pan' },
            { mode: 'draw' as ToolMode, label: '\u270D', title: 'Draw Wall' },
            { mode: 'erase' as ToolMode, label: '\u2716', title: 'Erase Wall' },
            { mode: 'set-start' as ToolMode, label: '\u25B6', title: 'Set Start' },
            { mode: 'set-end' as ToolMode, label: '\u25A0', title: 'Set End' },
          ]).map((tool) => (
            <button
              title={tool.title}
              onClick={() => props.store.setToolMode(tool.mode)}
              style={{
                width: '36px',
                height: '36px',
                'border-radius': '6px',
                'font-size': '16px',
                display: 'flex',
                'align-items': 'center',
                'justify-content': 'center',
                cursor: 'pointer',
                transition: 'all 0.15s',
                background: props.store.toolMode() === tool.mode ? 'rgba(0,212,170,0.12)' : 'var(--bg)',
                border: props.store.toolMode() === tool.mode ? '1px solid var(--cyan)' : '1px solid var(--border)',
                color: props.store.toolMode() === tool.mode ? 'var(--cyan)' : 'var(--text2)',
              }}
            >
              {tool.label}
            </button>
          ))}
        </div>

        {/* Heatmap toggle */}
        <div style={{ 'margin-top': '10px' }}>
          <label style={{
            display: 'flex',
            'align-items': 'center',
            gap: '8px',
            cursor: 'pointer',
            'font-size': '12px',
            color: props.store.heatmapEnabled() ? 'var(--cyan)' : 'var(--text2)',
            transition: 'color 0.15s',
            'user-select': 'none',
          }}>
            <div
              onClick={() => props.store.setHeatmapEnabled(!props.store.heatmapEnabled())}
              style={{
                width: '32px',
                height: '18px',
                'border-radius': '9px',
                background: props.store.heatmapEnabled() ? 'var(--cyan)' : 'var(--bg3)',
                border: `1px solid ${props.store.heatmapEnabled() ? 'var(--cyan)' : 'var(--border)'}`,
                position: 'relative',
                transition: 'all 0.2s',
                cursor: 'pointer',
                'flex-shrink': '0',
              }}
            >
              <div style={{
                width: '12px',
                height: '12px',
                'border-radius': '50%',
                background: props.store.heatmapEnabled() ? 'var(--bg)' : 'var(--text3)',
                position: 'absolute',
                top: '2px',
                left: props.store.heatmapEnabled() ? '16px' : '2px',
                transition: 'all 0.2s',
              }} />
            </div>
            <span onClick={() => props.store.setHeatmapEnabled(!props.store.heatmapEnabled())}>
              {t('heatmap')}
            </span>
          </label>
        </div>
      </div>

      {/* Action Buttons */}
      <div style={sectionStyle}>
        <div style={{ display: 'flex', 'flex-direction': 'column', gap: '6px', 'margin-top': '4px' }}>
          <button
            onClick={props.onGenerate}
            style={{
              padding: '9px 0',
              'border-radius': '6px',
              'font-size': '13px',
              'font-weight': 600,
              cursor: 'pointer',
              transition: 'all 0.15s',
              'text-align': 'center',
              width: '100%',
              'font-family': 'var(--font-sans)',
              background: 'rgba(0,212,170,0.1)',
              border: '1px solid var(--cyan)',
              color: 'var(--cyan)',
            }}
          >
            {t('generate')}
          </button>
          <button
            onClick={props.onSolve}
            style={{
              padding: '9px 0',
              'border-radius': '6px',
              'font-size': '13px',
              'font-weight': 600,
              cursor: 'pointer',
              transition: 'all 0.15s',
              'text-align': 'center',
              width: '100%',
              'font-family': 'var(--font-sans)',
              background: 'rgba(123,97,255,0.1)',
              border: '1px solid var(--purple)',
              color: 'var(--purple)',
            }}
          >
            {props.store.compareMode() ? t('compare') : t('solve')}
          </button>
          <button
            onClick={props.onAutoCompare}
            style={{
              padding: '9px 0',
              'border-radius': '6px',
              'font-size': '13px',
              'font-weight': 600,
              cursor: 'pointer',
              transition: 'all 0.15s',
              'text-align': 'center',
              width: '100%',
              'font-family': 'var(--font-sans)',
              background: 'rgba(240,180,41,0.08)',
              border: '1px solid var(--amber)',
              color: 'var(--amber)',
            }}
          >
            {t('autoCompareAll')}
          </button>
          <button
            onClick={props.onReset}
            style={{
              padding: '9px 0',
              'border-radius': '6px',
              'font-size': '13px',
              'font-weight': 600,
              cursor: 'pointer',
              transition: 'all 0.15s',
              'text-align': 'center',
              width: '100%',
              'font-family': 'var(--font-sans)',
              background: 'var(--bg)',
              border: '1px solid var(--border)',
              color: 'var(--text2)',
            }}
          >
            {t('reset')}
          </button>
        </div>
      </div>

      {/* Export */}
      <ExportMenu
        store={props.store}
        onRecordVideo={props.onRecordVideo}
        isRecording={() => props.store.isRecording()}
      />
    </aside>
  );
};

export default Sidebar;
