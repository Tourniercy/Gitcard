import { afterEach, describe, expect, it, vi } from 'vitest';
import { createPatPool } from './pat-pool';

describe('createPatPool', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it('throws if no tokens provided', () => {
    expect(() => createPatPool([])).toThrow('At least one PAT is required');
  });

  it('returns tokens in round-robin order', () => {
    const pool = createPatPool(['a', 'b', 'c']);
    expect(pool.getNextToken()).toBe('a');
    expect(pool.getNextToken()).toBe('b');
    expect(pool.getNextToken()).toBe('c');
    expect(pool.getNextToken()).toBe('a');
  });

  it('works with a single token', () => {
    const pool = createPatPool(['only']);
    expect(pool.getNextToken()).toBe('only');
    expect(pool.getNextToken()).toBe('only');
  });

  it('skips exhausted tokens', () => {
    const pool = createPatPool(['a', 'b', 'c']);
    pool.markExhausted('b');
    expect(pool.getNextToken()).toBe('a');
    expect(pool.getNextToken()).toBe('c');
    expect(pool.getNextToken()).toBe('a');
  });

  it('throws when all tokens are exhausted', () => {
    const pool = createPatPool(['a']);
    pool.markExhausted('a');
    expect(() => pool.getNextToken()).toThrow('All PAT tokens are rate-limited');
  });

  it('resets exhausted tokens after cooldown', () => {
    vi.useFakeTimers();
    const pool = createPatPool(['a']);
    pool.markExhausted('a');
    vi.advanceTimersByTime(60 * 60 * 1000); // 1 hour
    expect(pool.getNextToken()).toBe('a');
    vi.useRealTimers();
  });
});
