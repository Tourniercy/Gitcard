import * as Sentry from '@sentry/node';
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
      return Sentry.startSpan(
        { name: 'cache.get', op: 'cache.get', attributes: { 'cache.key': key } },
        async () => {
          const value = await cache.get<string>(key);
          if (value !== undefined) {
            cacheCounters.hits++;
            Sentry.metrics.count('cache_hits_total');
            return value;
          }
          cacheCounters.misses++;
          Sentry.metrics.count('cache_misses_total');
          return null;
        },
      );
    },

    async set(key: string, value: string, ttlSeconds: number): Promise<void> {
      return Sentry.startSpan(
        { name: 'cache.set', op: 'cache.set', attributes: { 'cache.key': key } },
        async () => {
          await cache.set(key, value, ttlSeconds * 1000);
        },
      );
    },
  };
}
