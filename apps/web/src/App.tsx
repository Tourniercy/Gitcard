import { useState, useEffect, useRef, useCallback } from 'react';
import { useCardConfig } from './hooks/useCardConfig';
import type { CardConfig, CardType } from './hooks/useCardConfig';
import { Sidebar } from './components/Sidebar';
import { CardList } from './components/CardList';
import { EmbedOutput } from './components/EmbedOutput';
import './styles/app.css';

export function App() {
  const {
    config,
    setUsername,
    setCards,
    toggleCard,
    setTheme,
    setOption,
    buildQueryString,
    cardPaths,
  } = useCardConfig();

  const [debouncedUsername, setDebouncedUsername] = useState('');
  const [isValid, setIsValid] = useState(false);
  const [isChecking, setIsChecking] = useState(false);
  const [hasChecked, setHasChecked] = useState(false);
  const debounceRef = useRef<ReturnType<typeof setTimeout>>(null);

  // Debounce username input
  useEffect(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }

    if (!config.username.trim()) {
      setDebouncedUsername('');
      setIsValid(false);
      setHasChecked(false);
      return;
    }

    debounceRef.current = setTimeout(() => {
      setDebouncedUsername(config.username.trim());
    }, 300);

    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, [config.username]);

  // Validate username via API
  useEffect(() => {
    if (!debouncedUsername) {
      return;
    }

    let cancelled = false;
    setIsChecking(true);

    fetch(`/api/data/${encodeURIComponent(debouncedUsername)}`)
      .then((res) => {
        if (!cancelled) {
          setIsValid(res.ok);
          setHasChecked(true);
          setIsChecking(false);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setIsValid(false);
          setHasChecked(true);
          setIsChecking(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [debouncedUsername]);

  const buildSrc = useCallback(
    (card: CardType): string => {
      return `${cardPaths[card]}${buildQueryString(card)}`;
    },
    [cardPaths, buildQueryString],
  );

  const handleToggle = useCallback(
    (key: string, value: boolean) => {
      setOption(key as keyof CardConfig, value as CardConfig[keyof CardConfig]);
    },
    [setOption],
  );

  const handleColorChange = useCallback(
    (key: string, value: string) => {
      setOption(key as keyof CardConfig, value as CardConfig[keyof CardConfig]);
    },
    [setOption],
  );

  const baseUrl = typeof window !== 'undefined' ? window.location.origin : '';

  const showPreview = isValid && debouncedUsername;
  const enabledCards = config.cards;

  return (
    <div className="app">
      <header className="app-header">
        <div className="header-content">
          <h1 className="header-title">GitCard</h1>
          <p className="header-subtitle">GitHub Stats Card Configurator</p>
        </div>
        <a
          className="header-link"
          href="https://github.com"
          target="_blank"
          rel="noopener noreferrer"
        >
          GitHub
        </a>
      </header>

      <div className="app-body">
        <Sidebar
          config={config}
          onUsernameChange={setUsername}
          onToggleCard={toggleCard}
          onThemeChange={setTheme}
          onToggle={handleToggle}
          onColorChange={handleColorChange}
        />

        <main className="main-content">
          {!config.username.trim() && (
            <div className="preview-placeholder">
              <p>Enter a GitHub username to get started</p>
            </div>
          )}

          {config.username.trim() && isChecking && (
            <div className="preview-placeholder">
              <p>Checking username...</p>
            </div>
          )}

          {config.username.trim() && hasChecked && !isValid && !isChecking && (
            <div className="preview-placeholder preview-error">
              <p>User not found</p>
            </div>
          )}

          {showPreview && (
            <>
              <CardList cards={enabledCards} onReorder={setCards} buildSrc={buildSrc} />
              <EmbedOutput cards={enabledCards} buildSrc={buildSrc} baseUrl={baseUrl} />
            </>
          )}
        </main>
      </div>
    </div>
  );
}
