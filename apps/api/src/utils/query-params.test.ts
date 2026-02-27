import { describe, expect, it } from 'vitest';
import { parseCardOptions } from './query-params';

describe('parseCardOptions', () => {
  it('returns defaults for empty query', () => {
    const result = parseCardOptions({});
    expect(result).toEqual({
      theme: 'default',
      hide: [],
      showIcons: true,
      hideBorder: false,
      hideTitle: false,
      cacheSeconds: 14400,
      locale: 'en',
    });
  });

  it('parses theme', () => {
    const result = parseCardOptions({ theme: 'dark' });
    expect(result.theme).toBe('dark');
  });

  it('parses hide as comma-separated list', () => {
    const result = parseCardOptions({ hide: 'stars,forks' });
    expect(result.hide).toEqual(['stars', 'forks']);
  });

  it('parses boolean flags', () => {
    const result = parseCardOptions({
      show_icons: 'false',
      hide_border: 'true',
      hide_title: 'true',
    });
    expect(result.showIcons).toBe(false);
    expect(result.hideBorder).toBe(true);
    expect(result.hideTitle).toBe(true);
  });

  it('parses color overrides', () => {
    const result = parseCardOptions({ bg_color: 'ff0000', text_color: '00ff00' });
    expect(result.bgColor).toBe('ff0000');
    expect(result.textColor).toBe('00ff00');
  });

  it('enforces minimum cache_seconds of 1800', () => {
    const result = parseCardOptions({ cache_seconds: '100' });
    expect(result.cacheSeconds).toBe(1800);
  });

  it('returns default cache_seconds for non-numeric value', () => {
    const result = parseCardOptions({ cache_seconds: 'not-a-number' });
    expect(result.cacheSeconds).toBe(14400);
  });
});
