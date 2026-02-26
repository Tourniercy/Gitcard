import { describe, expect, it } from 'vitest';
import { renderStatsCard } from './stats-card';
import type { GitHubStats, CardOptions } from '../types';

const mockStats: GitHubStats = {
  username: 'testuser',
  name: 'Test User',
  totalStars: 1234,
  totalForks: 567,
  totalCommits: 2345,
  totalPRs: 89,
  totalIssues: 45,
  contributedTo: 12,
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

describe('renderStatsCard', () => {
  it('returns valid SVG', () => {
    const svg = renderStatsCard(mockStats, defaultOptions);
    expect(svg).toContain('<svg');
    expect(svg).toContain('</svg>');
    expect(svg).toContain('495');
    expect(svg).toContain('195');
  });

  it('includes username in title', () => {
    const svg = renderStatsCard(mockStats, defaultOptions);
    expect(svg).toContain('Test User');
  });

  it('includes stat values', () => {
    const svg = renderStatsCard(mockStats, defaultOptions);
    expect(svg).toContain('1,234');
    expect(svg).toContain('2,345');
  });

  it('hides title when hideTitle is true', () => {
    const svg = renderStatsCard(mockStats, { ...defaultOptions, hideTitle: true });
    expect(svg).not.toContain("Test User&#39;s GitHub Stats");
  });

  it('hides specified stats', () => {
    const svg = renderStatsCard(mockStats, { ...defaultOptions, hide: ['stars'] });
    expect(svg).not.toContain('1,234');
  });

  it('includes grade ring', () => {
    const svg = renderStatsCard(mockStats, defaultOptions);
    expect(svg).toContain('<circle');
  });
});
