import { Component } from 'solid-js';
import { MazeState } from '../../stores/maze';
import { t } from '../../i18n';

const PlaybackControls: Component<{
  store: MazeState;
  onPause: () => void;
  onStep: () => void;
  onStepBack: () => void;
  onReset: () => void;
}> = (props) => {
  const btnBase: Record<string, string> = {
    height: '34px',
    'border-radius': '8px',
    display: 'flex',
    'align-items': 'center',
    'justify-content': 'center',
    background: 'var(--bg3)',
    border: '1px solid var(--border)',
    color: 'var(--text2)',
    cursor: 'pointer',
    'font-size': '13px',
    'font-family': 'var(--font-mono)',
    transition: 'all 0.15s',
    padding: '0 14px',
    gap: '4px',
  };

  const isRunning = () => {
    const s = props.store.playbackState();
    return s === 'generating' || s === 'solving';
  };

  const isPaused = () => {
    const s = props.store.playbackState();
    return s === 'gen-paused' || s === 'solve-paused';
  };

  const isActive = () => isRunning() || isPaused();

  const stateLabel = () => {
    const s = props.store.playbackState();
    switch (s) {
      case 'generating': return t('generating');
      case 'gen-paused': return t('pausedGen');
      case 'solving': return t('solvingState');
      case 'solve-paused': return t('pausedSolve');
      case 'done': return t('done');
      default: return t('ready');
    }
  };

  return (
    <div class="playback-bar" style={{
      height: '48px',
      background: 'var(--bg2)',
      'border-top': '1px solid var(--border)',
      display: 'flex',
      'align-items': 'center',
      padding: '0 16px',
      gap: '6px',
      'flex-shrink': '0',
    }}>
      {/* Pause / Resume */}
      <button
        onClick={props.onPause}
        disabled={!isActive()}
        style={{
          ...btnBase,
          ...(isRunning() ? {
            background: 'rgba(240,180,41,0.1)',
            'border-color': 'var(--amber)',
            color: 'var(--amber)',
          } : isPaused() ? {
            background: 'rgba(0,212,170,0.1)',
            'border-color': 'var(--cyan)',
            color: 'var(--cyan)',
          } : {}),
          opacity: isActive() ? '1' : '0.4',
        }}
      >
        {isRunning() ? '\u23F8 ' + t('pause') : '\u25B6 ' + t('resume')}
      </button>

      {/* Step back */}
      <button
        onClick={props.onStepBack}
        disabled={!isPaused()}
        style={{
          ...btnBase,
          opacity: isPaused() ? '1' : '0.4',
        }}
      >
        {'|\u25C0'} {t('stepBack')}
      </button>

      {/* Step forward */}
      <button
        onClick={props.onStep}
        disabled={!isPaused()}
        style={{
          ...btnBase,
          opacity: isPaused() ? '1' : '0.4',
        }}
      >
        {t('step')} {'\u25B6|'}
      </button>

      {/* Reset */}
      <button
        onClick={props.onReset}
        style={btnBase}
      >
        {t('reset')}
      </button>

      {/* State indicator */}
      <div style={{
        'margin-left': 'auto',
        'font-family': 'var(--font-mono)',
        'font-size': '11px',
        color: isRunning() ? 'var(--cyan)' : isPaused() ? 'var(--amber)' : 'var(--text3)',
        background: isActive() ? 'rgba(0,212,170,0.06)' : 'transparent',
        padding: '4px 10px',
        'border-radius': '4px',
      }}>
        {stateLabel()}
      </div>
    </div>
  );
};

export default PlaybackControls;
