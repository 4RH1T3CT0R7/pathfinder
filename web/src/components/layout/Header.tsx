import { Component, createSignal } from 'solid-js';
import { t, locale, setLocale } from '../../i18n';
import type { MazeState } from '../../stores/maze';
import { copyShareUrl } from '../../hooks/useShareUrl';

const Header: Component<{ onToggleSidebar?: () => void; store?: MazeState }> = (props) => {
  const [showCopied, setShowCopied] = createSignal(false);
  let copiedTimeout: number | undefined;

  const handleShare = async () => {
    if (!props.store) return;
    try {
      await copyShareUrl(props.store);
      setShowCopied(true);
      if (copiedTimeout !== undefined) clearTimeout(copiedTimeout);
      copiedTimeout = window.setTimeout(() => setShowCopied(false), 2000);
    } catch {
      // Clipboard API may not be available in some contexts
    }
  };

  return (
    <header style={{
      height: '48px',
      background: 'var(--bg2)',
      'border-bottom': '1px solid var(--border)',
      display: 'flex',
      'align-items': 'center',
      padding: '0 20px',
      gap: '12px',
      'flex-shrink': '0',
    }}>
      {/* Hamburger menu - mobile only */}
      <button
        class="hamburger-btn"
        onClick={() => props.onToggleSidebar?.()}
        style={{
          display: 'none',
          background: 'none',
          border: '1px solid var(--border)',
          'border-radius': '6px',
          color: 'var(--text2)',
          width: '34px',
          height: '34px',
          'align-items': 'center',
          'justify-content': 'center',
          cursor: 'pointer',
          'font-size': '18px',
          'flex-shrink': '0',
        }}
        aria-label="Toggle sidebar"
      >
        {'\u2630'}
      </button>

      <div style={{
        'font-family': 'var(--font-mono)',
        'font-size': '15px',
        'font-weight': 700,
        color: 'var(--cyan)',
        'letter-spacing': '0.03em',
      }}>
        {'> ' + t('title')}
        <span style={{ animation: 'blink 1s step-end infinite' }}>_</span>
      </div>
      <div class="header-subtitle" style={{
        'font-size': '11px',
        color: 'var(--text3)',
        'font-family': 'var(--font-mono)',
        background: 'var(--bg3)',
        padding: '3px 8px',
        'border-radius': '4px',
      }}>
        {t('subtitle')}
      </div>

      {/* Spacer */}
      <div style={{ flex: '1' }} />

      {/* Share button */}
      {props.store && (
        <div style={{ position: 'relative', 'flex-shrink': '0' }}>
          <button
            onClick={handleShare}
            style={{
              padding: '4px 12px',
              'border-radius': '6px',
              'font-size': '11px',
              'font-weight': 600,
              cursor: 'pointer',
              transition: 'all 0.15s',
              'font-family': 'var(--font-mono)',
              background: 'var(--bg3)',
              border: '1px solid var(--border)',
              color: 'var(--text2)',
              display: 'flex',
              'align-items': 'center',
              gap: '5px',
              'letter-spacing': '0.05em',
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.borderColor = 'var(--blue)';
              e.currentTarget.style.color = 'var(--blue)';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.borderColor = 'var(--border)';
              e.currentTarget.style.color = 'var(--text2)';
            }}
          >
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" />
              <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" />
            </svg>
            {t('share')}
          </button>

          {showCopied() && (
            <div style={{
              position: 'absolute',
              top: 'calc(100% + 8px)',
              right: '0',
              padding: '4px 10px',
              'border-radius': '4px',
              'font-size': '11px',
              'font-family': 'var(--font-mono)',
              background: 'var(--cyan)',
              color: 'var(--bg)',
              'font-weight': 600,
              'white-space': 'nowrap',
              'pointer-events': 'none',
              'z-index': '100',
            }}>
              {t('copied')}
            </div>
          )}
        </div>
      )}

      {/* Language toggle */}
      <button
        onClick={() => setLocale(locale() === 'en' ? 'ru' : 'en')}
        style={{
          background: 'var(--bg3)',
          border: '1px solid var(--border)',
          'border-radius': '6px',
          color: 'var(--text2)',
          'font-family': 'var(--font-mono)',
          'font-size': '11px',
          'font-weight': 600,
          padding: '4px 10px',
          cursor: 'pointer',
          transition: 'all 0.15s',
          'letter-spacing': '0.05em',
          'flex-shrink': '0',
        }}
      >
        {locale() === 'en' ? 'RU' : 'EN'}
      </button>
    </header>
  );
};

export default Header;
