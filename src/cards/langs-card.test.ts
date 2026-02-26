import { describe, expect, it } from 'vitest';
import { renderLangsCard } from './langs-card.js';
import type { TopLangsData, CardOptions } from '../types.js';

const mockLangs: TopLangsData = {
  username: 'testuser',
  languages: [
    { name: 'TypeScript', percentage: 42.3, color: '#3178c6', size: 42300 },
    { name: 'Python', percentage: 28.1, color: '#3572A5', size: 28100 },
    { name: 'Rust', percentage: 15.4, color: '#dea584', size: 15400 },
    { name: 'Go', percentage: 8.7, color: '#00ADD8', size: 8700 },
    { name: 'Shell', percentage: 5.5, color: '#89e051', size: 5500 },
  ],
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

describe('renderLangsCard', () => {
  it('returns valid SVG', () => {
    const svg = renderLangsCard(mockLangs, defaultOptions);
    expect(svg).toContain('<svg');
    expect(svg).toContain('</svg>');
  });

  it('includes language names', () => {
    const svg = renderLangsCard(mockLangs, defaultOptions);
    expect(svg).toContain('TypeScript');
    expect(svg).toContain('Python');
    expect(svg).toContain('Rust');
  });

  it('includes percentages', () => {
    const svg = renderLangsCard(mockLangs, defaultOptions);
    expect(svg).toContain('42.3%');
    expect(svg).toContain('28.1%');
  });

  it('includes donut chart paths', () => {
    const svg = renderLangsCard(mockLangs, defaultOptions);
    expect(svg).toMatch(/<path[^>]*fill="#3178c6"/);
  });

  it('handles empty languages', () => {
    const emptyLangs: TopLangsData = { username: 'test', languages: [] };
    const svg = renderLangsCard(emptyLangs, defaultOptions);
    expect(svg).toContain('<svg');
  });
});
