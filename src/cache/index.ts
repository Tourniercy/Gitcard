import { createMemoryCache } from './memory';
import { createRedisCache } from './redis';

export interface Cache {
  get(key: string): Promise<string | null>;
  set(key: string, value: string, ttlSeconds: number): Promise<void>;
}

export function createCache(redisUrl?: string): Cache {
  if (redisUrl) {
    return createRedisCache(redisUrl);
  }
  return createMemoryCache();
}

export { createMemoryCache } from './memory';
export { createRedisCache } from './redis';
