import { describe, expect, it } from 'vitest';
import { createArcPath, createGlassFilter, formatNumber } from './base-card';

describe('formatNumber', () => {
  it('formats with default locale', () => {
    expect(formatNumber(1234, 'en')).toBe('1,234');
  });

  it('formats zero', () => {
    expect(formatNumber(0, 'en')).toBe('0');
  });

  it('formats large numbers', () => {
    expect(formatNumber(1234567, 'en')).toBe('1,234,567');
  });
});

describe('createArcPath', () => {
  it('returns a valid SVG path d attribute', () => {
    const d = createArcPath(50, 50, 40, 0, Math.PI);
    expect(d).toContain('M');
    expect(d).toContain('A');
  });

  it('handles full circle', () => {
    const d = createArcPath(50, 50, 40, 0, Math.PI * 2 - 0.001);
    expect(d).toContain('A');
  });
});

describe('createGlassFilter', () => {
  it('returns SVG filter definition', () => {
    const filter = createGlassFilter('glass-1');
    expect(filter).toContain('<filter');
    expect(filter).toContain('id="glass-1"');
    expect(filter).toContain('feGaussianBlur');
  });
});
