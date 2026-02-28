import type { Theme } from './themes';
import {
  defaultTheme,
  flatTheme,
  breezyTheme,
  roseTheme,
  mintTheme,
  sandTheme,
  lavenderTheme,
  darkTheme,
  dimTheme,
  midnightTheme,
  onyxTheme,
  draculaTheme,
  monokaiTheme,
  nordTheme,
  solarizedTheme,
  solarizedDarkTheme,
  gruvboxTheme,
  gruvboxLightTheme,
  tokyoNightTheme,
  oneDarkTheme,
  oneLightTheme,
  catppuccinMochaTheme,
  catppuccinLatteTheme,
  synthwaveTheme,
  cobaltTheme,
  nightOwlTheme,
  auroraTheme,
  oceanTheme,
  forestTheme,
  sunsetTheme,
  cherryTheme,
  slateTheme,
  cottonCandyTheme,
  highContrastTheme,
  paperTheme,
} from './themes';

export type { Theme } from './themes';

const themes: Record<string, Theme> = {
  default: defaultTheme,
  flat: flatTheme,
  breezy: breezyTheme,
  rose: roseTheme,
  mint: mintTheme,
  sand: sandTheme,
  lavender: lavenderTheme,
  dark: darkTheme,
  dim: dimTheme,
  midnight: midnightTheme,
  onyx: onyxTheme,
  dracula: draculaTheme,
  monokai: monokaiTheme,
  nord: nordTheme,
  solarized: solarizedTheme,
  'solarized-dark': solarizedDarkTheme,
  gruvbox: gruvboxTheme,
  'gruvbox-light': gruvboxLightTheme,
  'tokyo-night': tokyoNightTheme,
  'one-dark': oneDarkTheme,
  'one-light': oneLightTheme,
  'catppuccin-mocha': catppuccinMochaTheme,
  'catppuccin-latte': catppuccinLatteTheme,
  synthwave: synthwaveTheme,
  cobalt: cobaltTheme,
  'night-owl': nightOwlTheme,
  aurora: auroraTheme,
  ocean: oceanTheme,
  forest: forestTheme,
  sunset: sunsetTheme,
  cherry: cherryTheme,
  slate: slateTheme,
  'cotton-candy': cottonCandyTheme,
  'high-contrast': highContrastTheme,
  paper: paperTheme,
};

export const THEME_NAMES: string[] = Object.keys(themes);

export interface ThemeGroup {
  label: string;
  themes: string[];
}

export const THEME_GROUPS: ThemeGroup[] = [
  {
    label: 'Light',
    themes: [
      'default',
      'flat',
      'breezy',
      'rose',
      'mint',
      'sand',
      'lavender',
      'solarized',
      'gruvbox-light',
      'one-light',
      'catppuccin-latte',
      'cotton-candy',
      'paper',
    ],
  },
  {
    label: 'Dark',
    themes: ['dark', 'dim', 'midnight', 'onyx', 'slate'],
  },
  {
    label: 'Editor',
    themes: [
      'dracula',
      'monokai',
      'nord',
      'solarized-dark',
      'gruvbox',
      'tokyo-night',
      'one-dark',
      'catppuccin-mocha',
      'synthwave',
      'cobalt',
      'night-owl',
    ],
  },
  {
    label: 'Atmosphere',
    themes: ['aurora', 'ocean', 'forest', 'sunset', 'cherry'],
  },
  {
    label: 'Accessibility',
    themes: ['high-contrast'],
  },
];

export function getTheme(name: string): Theme {
  return themes[name] ?? defaultTheme;
}
