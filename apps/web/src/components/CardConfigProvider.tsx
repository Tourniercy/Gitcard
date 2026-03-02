import { createContext, useContext, useCallback } from 'react';
import { useDebounceValue } from 'usehooks-ts';
import { useCardConfig } from '@/hooks/useCardConfig';
import { useGitHubData } from '@/hooks/useGitHubData';
import type { CardConfig, CardType } from '@/hooks/useCardConfig';
import type { GitHubData } from '@/hooks/useGitHubData';
import type { CardOptions } from '@gitcard/svg-renderer';

const ALL_CARDS: CardType[] = ['stats', 'streak', 'top-langs', 'profile'];

export interface CardConfigContextValue {
  config: CardConfig;
  setUsername: (username: string) => void;
  setTheme: (theme: string) => void;
  setOption: <K extends keyof CardConfig>(key: K, value: CardConfig[K]) => void;
  handleToggle: (key: string, value: boolean) => void;
  buildSrc: (card: CardType) => string;
  cardOptions: CardOptions;
  allCards: CardType[];
  data: GitHubData | undefined;
  isPending: boolean;
  error: Error | null;
}

const CardConfigContext = createContext<CardConfigContextValue | null>(null);

export function CardConfigProvider({ children }: { children: React.ReactNode }) {
  const { config, setUsername, setTheme, setOption, buildQueryString, cardPaths, cardOptions } =
    useCardConfig();

  const [debouncedUsername] = useDebounceValue(config.username.trim(), 300);
  const { data, isPending, error } = useGitHubData(debouncedUsername);

  const buildSrc = useCallback(
    (card: CardType): string => `${cardPaths[card]}${buildQueryString(card)}`,
    [cardPaths, buildQueryString],
  );

  const handleToggle = useCallback(
    (key: string, value: boolean) => {
      setOption(key as keyof CardConfig, value as CardConfig[keyof CardConfig]);
    },
    [setOption],
  );

  return (
    <CardConfigContext.Provider
      value={{
        config,
        setUsername,
        setTheme,
        setOption,
        handleToggle,
        buildSrc,
        cardOptions,
        allCards: ALL_CARDS,
        data,
        isPending,
        error,
      }}
    >
      {children}
    </CardConfigContext.Provider>
  );
}

export function useCardConfigContext(): CardConfigContextValue {
  const ctx = useContext(CardConfigContext);
  if (!ctx) throw new Error('useCardConfigContext must be used within CardConfigProvider');
  return ctx;
}
