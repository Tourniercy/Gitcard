import Redis from 'ioredis';
import type { Cache } from './index.js';

export function createRedisCache(redisUrl: string): Cache {
  const client = new Redis(redisUrl);

  client.on('error', (err) => {
    console.error('[redis] Connection error:', err.message);
  });

  return {
    async get(key: string): Promise<string | null> {
      try {
        return await client.get(key);
      } catch {
        return null;
      }
    },

    async set(key: string, value: string, ttlSeconds: number): Promise<void> {
      try {
        await client.setex(key, ttlSeconds, value);
      } catch {
        // Silently fail — cache is best-effort
      }
    },
  };
}
