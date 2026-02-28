import { useState, useMemo, useCallback } from 'react';
import type { CardOptions } from '@gitcard/svg-renderer';

export type CardType = 'stats' | 'streak' | 'top-langs';

export interface CardConfig {
  username: string;
  cards: CardType[];
  theme: string;
  showIcons: boolean;
  hideBorder: boolean;
  hideTitle: boolean;
  bgColor: string;
  titleColor: string;
  textColor: string;
  iconColor: string;
  borderColor: string;
  locale: string;
  hide: string[];
  cacheSeconds: string;
}

interface UseCardConfigReturn {
  config: CardConfig;
  setUsername: (username: string) => void;
  setCards: (cards: CardType[]) => void;
  toggleCard: (card: CardType) => void;
  setTheme: (theme: string) => void;
  setOption: <K extends keyof CardConfig>(key: K, value: CardConfig[K]) => void;
  buildQueryString: (cardType: CardType) => string;
  cardPaths: Record<CardType, string>;
  cardOptions: CardOptions;
}

const DEFAULTS: Omit<CardConfig, 'username' | 'cards'> = {
  theme: 'default',
  showIcons: true,
  hideBorder: false,
  hideTitle: false,
  bgColor: '',
  titleColor: '',
  textColor: '',
  iconColor: '',
  borderColor: '',
  locale: 'en',
  hide: [],
  cacheSeconds: '',
};

export function useCardConfig(): UseCardConfigReturn {
  const [config, setConfig] = useState<CardConfig>({
    username: '',
    cards: ['stats', 'streak', 'top-langs'],
    ...DEFAULTS,
  });

  const setUsername = useCallback((username: string) => {
    setConfig((prev) => ({ ...prev, username }));
  }, []);

  const setCards = useCallback((cards: CardType[]) => {
    setConfig((prev) => ({ ...prev, cards }));
  }, []);

  const toggleCard = useCallback((card: CardType) => {
    setConfig((prev) => {
      const cards = prev.cards.includes(card)
        ? prev.cards.filter((c) => c !== card)
        : [...prev.cards, card];
      return { ...prev, cards };
    });
  }, []);

  const setTheme = useCallback((theme: string) => {
    setConfig((prev) => ({ ...prev, theme }));
  }, []);

  const setOption = useCallback(<K extends keyof CardConfig>(key: K, value: CardConfig[K]) => {
    setConfig((prev) => ({ ...prev, [key]: value }));
  }, []);

  const cardPaths = useMemo<Record<CardType, string>>(() => {
    const u = encodeURIComponent(config.username);
    return {
      stats: `/stats/${u}`,
      streak: `/stats/${u}/streak`,
      'top-langs': `/stats/${u}/top-langs`,
    };
  }, [config.username]);

  const buildQueryString = useCallback(
    (cardType: CardType): string => {
      const params = new URLSearchParams();

      if (config.theme !== 'default') {
        params.set('theme', config.theme);
      }
      if (!config.showIcons) {
        params.set('show_icons', 'false');
      }
      if (config.hideBorder) {
        params.set('hide_border', 'true');
      }
      if (config.hideTitle) {
        params.set('hide_title', 'true');
      }
      if (config.bgColor) {
        params.set('bg_color', config.bgColor);
      }
      if (config.titleColor) {
        params.set('title_color', config.titleColor);
      }
      if (config.textColor) {
        params.set('text_color', config.textColor);
      }
      if (config.iconColor) {
        params.set('icon_color', config.iconColor);
      }
      if (config.borderColor) {
        params.set('border_color', config.borderColor);
      }
      if (config.locale !== 'en') {
        params.set('locale', config.locale);
      }
      if (cardType === 'stats' && config.hide.length > 0) {
        params.set('hide', config.hide.join(','));
      }
      if (config.cacheSeconds && Number(config.cacheSeconds) >= 1800) {
        params.set('cache_seconds', config.cacheSeconds);
      }

      const qs = params.toString();
      return qs ? `?${qs}` : '';
    },
    [config],
  );

  const cardOptions = useMemo<CardOptions>(
    () => ({
      theme: config.theme,
      hide: config.hide,
      showIcons: config.showIcons,
      hideBorder: config.hideBorder,
      hideTitle: config.hideTitle,
      bgColor: config.bgColor || undefined,
      titleColor: config.titleColor || undefined,
      textColor: config.textColor || undefined,
      iconColor: config.iconColor || undefined,
      borderColor: config.borderColor || undefined,
      cacheSeconds: Number(config.cacheSeconds) || 14400,
      locale: config.locale,
    }),
    [config],
  );

  return {
    config,
    setUsername,
    setCards,
    toggleCard,
    setTheme,
    setOption,
    buildQueryString,
    cardPaths,
    cardOptions,
  };
}
