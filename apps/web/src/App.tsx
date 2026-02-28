import { useState, useEffect, useRef, useCallback } from 'react';
import { GitHubLogoIcon } from '@radix-ui/react-icons';
import { useCardConfig } from '@/hooks/useCardConfig';
import { useGitHubData } from '@/hooks/useGitHubData';
import type { CardConfig, CardType } from '@/hooks/useCardConfig';
import { Sidebar } from '@/components/Sidebar';
import { CardList } from '@/components/CardList';
import { ModeToggle } from '@/components/mode-toggle';
import { Button } from '@/components/ui/button';

const ALL_CARDS: CardType[] = ['stats', 'streak', 'top-langs'];

export function App() {
  const { config, setUsername, setTheme, setOption, buildQueryString, cardPaths, cardOptions } =
    useCardConfig();

  const [debouncedUsername, setDebouncedUsername] = useState('');
  const debounceRef = useRef<ReturnType<typeof setTimeout>>(null);

  useEffect(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }

    if (!config.username.trim()) {
      setDebouncedUsername('');
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

  const { data, isLoading, error } = useGitHubData(debouncedUsername);

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

  const showPreview = data !== null;

  return (
    <div className="flex min-h-screen flex-col">
      {/* Header */}
      <header className="sticky top-0 z-50 w-full border-b border-border/40 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="container flex h-14 items-center">
          <a href="/" className="mr-2 flex items-center md:mr-6 md:space-x-2">
            <span className="hidden font-bold md:inline-block">GitCard</span>
          </a>
          <nav className="flex flex-1 items-center md:justify-end">
            <Button variant="ghost" size="icon" className="size-8" asChild>
              <a
                aria-label="GitHub repo"
                href="https://github.com/Tourniercy/Gitcard"
                target="_blank"
                rel="noopener noreferrer"
              >
                <GitHubLogoIcon className="size-4" aria-hidden="true" />
              </a>
            </Button>
            <ModeToggle />
          </nav>
        </div>
      </header>

      {/* Body */}
      <div className="mx-auto grid w-full max-w-[1200px] flex-1 grid-cols-1 gap-6 p-6 md:grid-cols-[300px_1fr]">
        <Sidebar
          config={config}
          onUsernameChange={setUsername}
          onThemeChange={setTheme}
          onToggle={handleToggle}
        />

        <main className="flex min-w-0 flex-col gap-6">
          {!config.username.trim() && (
            <div className="flex min-h-[300px] items-center justify-center rounded-lg border-2 border-dashed">
              <p className="text-muted-foreground">Enter a GitHub username to get started</p>
            </div>
          )}

          {config.username.trim() && isLoading && (
            <div className="flex min-h-[300px] items-center justify-center rounded-lg border-2 border-dashed">
              <div className="flex flex-col items-center gap-2">
                <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent" />
                <p className="text-sm text-muted-foreground">Loading...</p>
              </div>
            </div>
          )}

          {config.username.trim() && error && !isLoading && (
            <div className="flex min-h-[300px] items-center justify-center rounded-lg border-2 border-dashed border-destructive bg-destructive/5">
              <p className="text-destructive">{error}</p>
            </div>
          )}

          {showPreview && (
            <CardList cards={ALL_CARDS} data={data} options={cardOptions} buildSrc={buildSrc} />
          )}
        </main>
      </div>
    </div>
  );
}
