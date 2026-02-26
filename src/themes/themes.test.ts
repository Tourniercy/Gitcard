import { describe, expect, it } from 'vitest';
import { getTheme, THEME_NAMES } from './index.js';

describe('theme system', () => {
  it('returns default theme', () => {
    const theme = getTheme('default');
    expect(theme.name).toBe('default');
    expect(theme.background).toBeDefined();
    expect(theme.text).toBeDefined();
  });

  it('returns dark theme', () => {
    const theme = getTheme('dark');
    expect(theme.name).toBe('dark');
  });

  it('returns dracula theme', () => {
    const theme = getTheme('dracula');
    expect(theme.name).toBe('dracula');
  });

  it('falls back to default for unknown theme', () => {
    const theme = getTheme('nonexistent');
    expect(theme.name).toBe('default');
  });

  it('lists all available theme names', () => {
    expect(THEME_NAMES).toContain('default');
    expect(THEME_NAMES).toContain('dark');
    expect(THEME_NAMES).toContain('dracula');
    expect(THEME_NAMES.length).toBe(3);
  });
});
