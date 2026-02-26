import type { Theme } from './themes';
import { darkTheme, defaultTheme, draculaTheme } from './themes';

export type { Theme } from './themes';

const themes: Record<string, Theme> = {
  default: defaultTheme,
  dark: darkTheme,
  dracula: draculaTheme,
};

export const THEME_NAMES: string[] = Object.keys(themes);

export function getTheme(name: string): Theme {
  return themes[name] ?? defaultTheme;
}
