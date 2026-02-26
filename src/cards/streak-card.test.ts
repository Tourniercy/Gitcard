import { describe, expect, it } from 'vitest';
import { renderStreakCard } from './streak-card';
import type { StreakData, CardOptions } from '../types';

const mockStreak: StreakData = {
  username: 'testuser',
  totalContributions: 2345,
  currentStreak: 47,
  longestStreak: 124,
  currentStreakStart: '2026-01-10',
  currentStreakEnd: '2026-02-26',
  longestStreakStart: '2024-05-01',
  longestStreakEnd: '2024-09-01',
};

const defaultOptions: CardOptions = {
  theme: 'default',
  hide: [],
  showIcons: true,
  hideBorder: false,
  hideTitle: false,
  cacheSeconds: 14400,
  locale: 'en',
};

describe('renderStreakCard', () => {
  it('returns valid SVG', () => {
    const svg = renderStreakCard(mockStreak, defaultOptions);
    expect(svg).toContain('<svg');
    expect(svg).toContain('</svg>');
  });

  it('includes streak numbers', () => {
    const svg = renderStreakCard(mockStreak, defaultOptions);
    expect(svg).toContain('47');
    expect(svg).toContain('124');
    expect(svg).toContain('2,345');
  });

  it('includes date ranges', () => {
    const svg = renderStreakCard(mockStreak, defaultOptions);
    expect(svg).toContain('Jan');
    expect(svg).toContain('Feb');
  });

  it('includes flame icon path', () => {
    const svg = renderStreakCard(mockStreak, defaultOptions);
    expect(svg).toContain('path');
  });
});
