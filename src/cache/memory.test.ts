import { describe, expect, it } from 'vitest';
import { createMemoryCache } from './memory.js';

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

describe('MemoryCache', () => {
  it('returns null for missing key', async () => {
    const cache = createMemoryCache();
    expect(await cache.get('missing')).toBeNull();
  });

  it('stores and retrieves a value', async () => {
    const cache = createMemoryCache();
    await cache.set('key', 'value', 3600);
    expect(await cache.get('key')).toBe('value');
  });

  it('expires entries after TTL elapses', async () => {
    const cache = createMemoryCache();
    await cache.set('key', 'value', 1); // 1 second TTL
    // Value should be available immediately
    expect(await cache.get('key')).toBe('value');
    // Wait for TTL to expire
    await delay(1100);
    expect(await cache.get('key')).toBeNull();
  });

  it('respects max entries limit', async () => {
    const cache = createMemoryCache(2);
    await cache.set('a', '1', 3600);
    await cache.set('b', '2', 3600);
    await cache.set('c', '3', 3600); // should evict 'a'
    expect(await cache.get('a')).toBeNull();
    expect(await cache.get('b')).toBe('2');
    expect(await cache.get('c')).toBe('3');
  });

  it('overwrites existing key', async () => {
    const cache = createMemoryCache();
    await cache.set('key', 'old', 3600);
    await cache.set('key', 'new', 3600);
    expect(await cache.get('key')).toBe('new');
  });
});
