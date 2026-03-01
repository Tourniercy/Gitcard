export interface Theme {
  name: string;
  background: string;
  border: string;
  title: string;
  text: string;
  muted: string;
  icon: string;
  ring: string;
}

// --- Light themes ---

export const defaultTheme: Theme = {
  name: 'default',
  background: 'rgba(255, 255, 255, 0.7)',

  border: 'rgba(0, 0, 0, 0.1)',
  title: '#1f2937',
  text: '#374151',
  muted: '#6b7280',
  icon: '#3b82f6',
  ring: '#3b82f6',
};

export const flatTheme: Theme = {
  name: 'flat',
  background: 'rgba(255, 255, 255, 1)',
  border: '#e5e7eb',
  title: '#111827',
  text: '#374151',
  muted: '#9ca3af',
  icon: '#6366f1',
  ring: '#6366f1',
};

export const breezyTheme: Theme = {
  name: 'breezy',
  background: 'rgba(240, 249, 255, 0.8)',
  border: 'rgba(56, 189, 248, 0.2)',
  title: '#0c4a6e',
  text: '#075985',
  muted: '#7dd3fc',
  icon: '#0ea5e9',
  ring: '#0ea5e9',
};

export const roseTheme: Theme = {
  name: 'rose',
  background: 'rgba(255, 241, 242, 0.8)',
  border: 'rgba(244, 63, 94, 0.15)',
  title: '#881337',
  text: '#9f1239',
  muted: '#fda4af',
  icon: '#f43f5e',
  ring: '#f43f5e',
};

export const mintTheme: Theme = {
  name: 'mint',
  background: 'rgba(236, 253, 245, 0.8)',
  border: 'rgba(16, 185, 129, 0.15)',
  title: '#064e3b',
  text: '#065f46',
  muted: '#6ee7b7',
  icon: '#10b981',
  ring: '#10b981',
};

export const sandTheme: Theme = {
  name: 'sand',
  background: 'rgba(255, 251, 235, 0.8)',
  border: 'rgba(217, 119, 6, 0.15)',
  title: '#78350f',
  text: '#92400e',
  muted: '#fcd34d',
  icon: '#d97706',
  ring: '#d97706',
};

export const lavenderTheme: Theme = {
  name: 'lavender',
  background: 'rgba(245, 243, 255, 0.8)',
  border: 'rgba(139, 92, 246, 0.15)',
  title: '#4c1d95',
  text: '#5b21b6',
  muted: '#c4b5fd',
  icon: '#8b5cf6',
  ring: '#8b5cf6',
};

// --- Dark themes ---

export const darkTheme: Theme = {
  name: 'dark',
  background: 'rgba(13, 17, 23, 0.8)',
  border: 'rgba(255, 255, 255, 0.1)',
  title: '#e6edf3',
  text: '#c9d1d9',
  muted: '#8b949e',
  icon: '#58a6ff',
  ring: '#58a6ff',
};

export const dimTheme: Theme = {
  name: 'dim',
  background: 'rgba(34, 39, 46, 0.85)',
  border: 'rgba(68, 76, 86, 0.6)',
  title: '#adbac7',
  text: '#768390',
  muted: '#545d68',
  icon: '#539bf5',
  ring: '#539bf5',
};

export const midnightTheme: Theme = {
  name: 'midnight',
  background: 'rgba(15, 23, 42, 0.85)',
  border: 'rgba(51, 65, 85, 0.5)',
  title: '#f1f5f9',
  text: '#cbd5e1',
  muted: '#64748b',
  icon: '#818cf8',
  ring: '#818cf8',
};

export const onyxTheme: Theme = {
  name: 'onyx',
  background: 'rgba(0, 0, 0, 0.9)',
  border: 'rgba(255, 255, 255, 0.08)',
  title: '#ffffff',
  text: '#a1a1aa',
  muted: '#52525b',
  icon: '#a78bfa',
  ring: '#a78bfa',
};

// --- Popular editor themes ---

export const draculaTheme: Theme = {
  name: 'dracula',
  background: 'rgba(40, 42, 54, 0.8)',
  border: 'rgba(98, 114, 164, 0.3)',
  title: '#f8f8f2',
  text: '#f8f8f2',
  muted: '#6272a4',
  icon: '#bd93f9',
  ring: '#bd93f9',
};

export const monokaiTheme: Theme = {
  name: 'monokai',
  background: 'rgba(39, 40, 34, 0.85)',
  border: 'rgba(117, 113, 94, 0.3)',
  title: '#f8f8f2',
  text: '#f8f8f2',
  muted: '#75715e',
  icon: '#a6e22e',
  ring: '#a6e22e',
};

export const nordTheme: Theme = {
  name: 'nord',
  background: 'rgba(46, 52, 64, 0.85)',
  border: 'rgba(76, 86, 106, 0.4)',
  title: '#eceff4',
  text: '#d8dee9',
  muted: '#4c566a',
  icon: '#88c0d0',
  ring: '#88c0d0',
};

export const solarizedTheme: Theme = {
  name: 'solarized',
  background: 'rgba(253, 246, 227, 0.85)',
  border: 'rgba(147, 161, 161, 0.2)',
  title: '#073642',
  text: '#586e75',
  muted: '#93a1a1',
  icon: '#268bd2',
  ring: '#268bd2',
};

export const solarizedDarkTheme: Theme = {
  name: 'solarized-dark',
  background: 'rgba(0, 43, 54, 0.85)',
  border: 'rgba(88, 110, 117, 0.3)',
  title: '#eee8d5',
  text: '#839496',
  muted: '#586e75',
  icon: '#268bd2',
  ring: '#268bd2',
};

export const gruvboxTheme: Theme = {
  name: 'gruvbox',
  background: 'rgba(40, 40, 40, 0.85)',
  border: 'rgba(168, 153, 132, 0.3)',
  title: '#ebdbb2',
  text: '#d5c4a1',
  muted: '#a89984',
  icon: '#fabd2f',
  ring: '#fabd2f',
};

export const gruvboxLightTheme: Theme = {
  name: 'gruvbox-light',
  background: 'rgba(251, 241, 199, 0.85)',
  border: 'rgba(168, 153, 132, 0.2)',
  title: '#3c3836',
  text: '#504945',
  muted: '#928374',
  icon: '#d79921',
  ring: '#d79921',
};

export const tokyoNightTheme: Theme = {
  name: 'tokyo-night',
  background: 'rgba(26, 27, 38, 0.85)',
  border: 'rgba(41, 46, 66, 0.5)',
  title: '#c0caf5',
  text: '#a9b1d6',
  muted: '#565f89',
  icon: '#7aa2f7',
  ring: '#7aa2f7',
};

export const oneDarkTheme: Theme = {
  name: 'one-dark',
  background: 'rgba(40, 44, 52, 0.85)',
  border: 'rgba(76, 82, 99, 0.4)',
  title: '#abb2bf',
  text: '#abb2bf',
  muted: '#5c6370',
  icon: '#61afef',
  ring: '#61afef',
};

export const oneLightTheme: Theme = {
  name: 'one-light',
  background: 'rgba(250, 250, 250, 0.85)',
  border: 'rgba(56, 58, 66, 0.1)',
  title: '#383a42',
  text: '#383a42',
  muted: '#a0a1a7',
  icon: '#4078f2',
  ring: '#4078f2',
};

export const catppuccinMochaTheme: Theme = {
  name: 'catppuccin-mocha',
  background: 'rgba(30, 30, 46, 0.85)',
  border: 'rgba(69, 71, 90, 0.5)',
  title: '#cdd6f4',
  text: '#bac2de',
  muted: '#6c7086',
  icon: '#cba6f7',
  ring: '#cba6f7',
};

export const catppuccinLatteTheme: Theme = {
  name: 'catppuccin-latte',
  background: 'rgba(239, 241, 245, 0.85)',
  border: 'rgba(172, 176, 190, 0.3)',
  title: '#4c4f69',
  text: '#5c5f77',
  muted: '#9ca0b0',
  icon: '#8839ef',
  ring: '#8839ef',
};

export const synthwaveTheme: Theme = {
  name: 'synthwave',
  background: 'rgba(36, 18, 59, 0.9)',
  border: 'rgba(148, 80, 199, 0.3)',
  title: '#f97e72',
  text: '#e8e3e3',
  muted: '#848bbd',
  icon: '#ff7edb',
  ring: '#ff7edb',
};

export const cobaltTheme: Theme = {
  name: 'cobalt',
  background: 'rgba(0, 28, 54, 0.9)',
  border: 'rgba(25, 66, 113, 0.5)',
  title: '#ffffff',
  text: '#e1efff',
  muted: '#7590ab',
  icon: '#ffc600',
  ring: '#ffc600',
};

export const nightOwlTheme: Theme = {
  name: 'night-owl',
  background: 'rgba(1, 22, 39, 0.9)',
  border: 'rgba(23, 62, 88, 0.5)',
  title: '#d6deeb',
  text: '#d6deeb',
  muted: '#637777',
  icon: '#7fdbca',
  ring: '#7fdbca',
};

export const auroraTheme: Theme = {
  name: 'aurora',
  background: 'rgba(7, 15, 43, 0.9)',
  border: 'rgba(76, 201, 240, 0.2)',
  title: '#f0f0f0',
  text: '#c8d6e5',
  muted: '#576574',
  icon: '#48dbfb',
  ring: '#48dbfb',
};

export const oceanTheme: Theme = {
  name: 'ocean',
  background: 'rgba(3, 28, 59, 0.9)',
  border: 'rgba(0, 105, 148, 0.3)',
  title: '#e0f7fa',
  text: '#b2ebf2',
  muted: '#4db6ac',
  icon: '#00bcd4',
  ring: '#00bcd4',
};

export const forestTheme: Theme = {
  name: 'forest',
  background: 'rgba(16, 36, 24, 0.9)',
  border: 'rgba(46, 125, 50, 0.3)',
  title: '#c8e6c9',
  text: '#a5d6a7',
  muted: '#66bb6a',
  icon: '#4caf50',
  ring: '#4caf50',
};

export const sunsetTheme: Theme = {
  name: 'sunset',
  background: 'rgba(54, 17, 31, 0.9)',
  border: 'rgba(239, 108, 0, 0.2)',
  title: '#fff3e0',
  text: '#ffe0b2',
  muted: '#ff8a65',
  icon: '#ff6d00',
  ring: '#ff6d00',
};

export const cherryTheme: Theme = {
  name: 'cherry',
  background: 'rgba(45, 10, 20, 0.9)',
  border: 'rgba(211, 47, 47, 0.25)',
  title: '#fce4ec',
  text: '#f8bbd0',
  muted: '#e57373',
  icon: '#ef5350',
  ring: '#ef5350',
};

export const slateTheme: Theme = {
  name: 'slate',
  background: 'rgba(30, 41, 59, 0.85)',
  border: 'rgba(71, 85, 105, 0.4)',
  title: '#f8fafc',
  text: '#e2e8f0',
  muted: '#64748b',
  icon: '#38bdf8',
  ring: '#38bdf8',
};

export const cottonCandyTheme: Theme = {
  name: 'cotton-candy',
  background: 'rgba(253, 242, 248, 0.85)',
  border: 'rgba(236, 72, 153, 0.15)',
  title: '#831843',
  text: '#9d174d',
  muted: '#f9a8d4',
  icon: '#ec4899',
  ring: '#ec4899',
};

export const highContrastTheme: Theme = {
  name: 'high-contrast',
  background: 'rgba(0, 0, 0, 1)',
  border: '#ffffff',
  title: '#ffffff',
  text: '#ffffff',
  muted: '#cccccc',
  icon: '#ffff00',
  ring: '#ffff00',
};

export const paperTheme: Theme = {
  name: 'paper',
  background: 'rgba(250, 248, 240, 0.9)',
  border: 'rgba(180, 166, 140, 0.25)',
  title: '#433422',
  text: '#5c4b37',
  muted: '#a09080',
  icon: '#8b6914',
  ring: '#8b6914',
};
