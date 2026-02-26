import { LRUCache } from 'lru-cache';
import type { Cache } from './index';

export function createMemoryCache(maxEntries = 500): Cache {
  const cache = new LRUCache<string, string>({
    max: maxEntries,
  });

  return {
    async get(key: string): Promise<string | null> {
      return cache.get(key) ?? null;
    },

    async set(key: string, value: string, ttlSeconds: number): Promise<void> {
      cache.set(key, value, { ttl: ttlSeconds * 1000 });
    },
  };
}
