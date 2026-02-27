import { createEnv } from '@t3-oss/env-core';
import { z } from 'zod';

const env = createEnv({
  server: {
    PORT: z.coerce.number().min(1).max(65535).default(3000),
    LOG_LEVEL: z.enum(['debug', 'info', 'warn', 'error']).default('info'),
    REDIS_URL: z.string().url().optional(),
    CACHE_TTL: z.coerce.number().min(60).default(14400),
    METRICS_TOKEN: z.string().min(1).optional(),
    PAT_1: z.string().min(1, 'At least PAT_1 must be set'),
  },
  runtimeEnv: process.env,
  emptyStringAsUndefined: true,
});

function collectPats(): string[] {
  const pats: string[] = [];
  for (let i = 1; ; i++) {
    const pat = process.env[`PAT_${i}`];
    if (!pat) break;
    pats.push(pat);
  }
  return pats;
}

export interface AppConfig {
  pats: string[];
  port: number;
  redisUrl?: string;
  cacheTtl: number;
  logLevel: 'debug' | 'info' | 'warn' | 'error';
  metricsToken?: string;
}

export function loadConfig(): AppConfig {
  return {
    pats: collectPats(),
    port: env.PORT,
    redisUrl: env.REDIS_URL,
    cacheTtl: env.CACHE_TTL,
    logLevel: env.LOG_LEVEL,
    metricsToken: env.METRICS_TOKEN,
  };
}
