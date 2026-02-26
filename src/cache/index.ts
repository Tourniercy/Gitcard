export interface Cache {
  get(key: string): Promise<string | null>;
  set(key: string, value: string, ttlSeconds: number): Promise<void>;
}

import { createMemoryCache } from './memory.js';
import { createRedisCache } from './redis.js';

export function createCache(redisUrl?: string): Cache {
  if (redisUrl) {
    return createRedisCache(redisUrl);
  }
  return createMemoryCache();
}

export { createMemoryCache } from './memory.js';
export { createRedisCache } from './redis.js';
