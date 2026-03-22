import { Component, For } from 'solid-js';
import type { MazeState, GeneratorAlgo } from '../../stores/maze';
import { t } from '../../i18n';

interface Preset {
  label: string;
  seed: number;
  width: number;
  height: number;
  generator: GeneratorAlgo;
}

const PRESETS: Preset[] = [
  { label: 'Tiny (5x5)',      seed: 42,    width: 5,   height: 5,   generator: 'dfs' },
  { label: 'Classic (20x20)', seed: 1337,  width: 20,  height: 20,  generator: 'dfs' },
  { label: 'Large (50x50)',   seed: 777,   width: 50,  height: 50,  generator: 'prim' },
  { label: 'Spiral (30x30)',  seed: 12345, width: 30,  height: 30,  generator: 'binary_tree' },
  { label: 'Dense (40x40)',   seed: 99,    width: 40,  height: 40,  generator: 'kruskal' },
  { label: 'Huge (100x100)',  seed: 42,    width: 100, height: 100, generator: 'wilson' },
];

const PresetPicker: Component<{
  store: MazeState;
  onApply: () => void;
}> = (props) => {
  const applyPreset = (preset: Preset) => {
    props.store.setSeed(preset.seed);
    props.store.setWidth(preset.width);
    props.store.setHeight(preset.height);
    props.store.setGeneratorAlgo(preset.generator);
    props.onApply();
  };

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
        {t('presets')}
      </div>
      <div style={{
        display: 'flex',
        'flex-wrap': 'wrap',
        gap: '4px',
      }}>
        <For each={PRESETS}>
          {(preset) => (
            <button
              onClick={() => applyPreset(preset)}
              style={{
                padding: '5px 10px',
                'border-radius': '4px',
                'font-size': '11px',
                'font-family': 'var(--font-mono)',
                cursor: 'pointer',
                transition: 'all 0.15s',
                background: 'var(--bg)',
                border: '1px solid var(--border)',
                color: 'var(--text2)',
                'white-space': 'nowrap',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.borderColor = 'var(--cyan)';
                e.currentTarget.style.color = 'var(--cyan)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.borderColor = 'var(--border)';
                e.currentTarget.style.color = 'var(--text2)';
              }}
            >
              {preset.label}
            </button>
          )}
        </For>
      </div>
    </div>
  );
};

export default PresetPicker;
