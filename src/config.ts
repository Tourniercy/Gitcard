import { z } from 'zod';

const configSchema = z.object({
  port: z.coerce.number().default(3000),
  redisUrl: z.string().url().optional(),
  cacheTtl: z.coerce.number().min(60).default(14400),
  logLevel: z.enum(['debug', 'info', 'warn', 'error']).default('info'),
});

export interface AppConfig {
  pats: string[];
  port: number;
  redisUrl?: string;
  cacheTtl: number;
  logLevel: 'debug' | 'info' | 'warn' | 'error';
}

function collectPats(): string[] {
  const pats: string[] = [];
  for (let i = 1; ; i++) {
    const pat = process.env[`PAT_${i}`];
    if (!pat) break;
    pats.push(pat);
  }
  return pats;
}

export function loadConfig(): AppConfig {
  const pats = collectPats();
  if (pats.length === 0) {
    throw new Error('At least PAT_1 must be set');
  }

  const parsed = configSchema.parse({
    port: process.env['PORT'],
    redisUrl: process.env['REDIS_URL'] || undefined,
    cacheTtl: process.env['CACHE_TTL'],
    logLevel: process.env['LOG_LEVEL'],
  });

  return {
    pats,
    ...parsed,
  };
}
