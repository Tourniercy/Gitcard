import { describe, expect, it } from 'vitest';
import { createCache } from './index';

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

describe('cache (cacheable)', () => {
  it('returns null for missing key', async () => {
    const cache = createCache();
    expect(await cache.get('missing')).toBeNull();
  });

  it('stores and retrieves a value', async () => {
    const cache = createCache();
    await cache.set('key', 'value', 3600);
    expect(await cache.get('key')).toBe('value');
  });

  it('expires entries after TTL elapses', async () => {
    const cache = createCache();
    await cache.set('key', 'value', 1); // 1 second TTL
    expect(await cache.get('key')).toBe('value');
    await delay(1100);
    expect(await cache.get('key')).toBeNull();
  });

  it('overwrites existing key', async () => {
    const cache = createCache();
    await cache.set('key', 'old', 3600);
    await cache.set('key', 'new', 3600);
    expect(await cache.get('key')).toBe('new');
  });
});
