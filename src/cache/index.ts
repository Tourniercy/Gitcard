import { Cacheable } from 'cacheable';
import KeyvRedis from '@keyv/redis';
import { cacheCounters } from '../metrics/store';

export interface Cache {
  get(key: string): Promise<string | null>;
  set(key: string, value: string, ttlSeconds: number): Promise<void>;
}

export function createCache(redisUrl?: string): Cache {
  const options: ConstructorParameters<typeof Cacheable>[0] = {
    nonBlocking: true,
  };

  if (redisUrl) {
    options.secondary = new KeyvRedis(redisUrl);
  }

  const cache = new Cacheable(options);

  return {
    async get(key: string): Promise<string | null> {
      const value = await cache.get<string>(key);
      if (value !== undefined) {
        cacheCounters.hits++;
        return value;
      }
      cacheCounters.misses++;
      return null;
    },

    async set(key: string, value: string, ttlSeconds: number): Promise<void> {
      await cache.set(key, value, ttlSeconds * 1000);
    },
  };
}
