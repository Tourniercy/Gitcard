# github-stats-cards Full Scaffold Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Scaffold the entire github-stats-cards project from zero to a working dev server and Docker deployment.

**Architecture:** Hono API server generating SVG stat cards with glassmorphism aesthetic. Hybrid SVG rendering (shared utilities in base-card.ts, monolithic layouts per card). Two-tier caching (LRU memory + Redis). GitHub GraphQL API with PAT token rotation.

**Tech Stack:** Node.js 24, Hono, TypeScript 5.8+, Vite 8 (beta/Rolldown), Zod, lru-cache, ioredis, Oxlint, Oxfmt, vitest, Docker

**Context7 Requirement:** Before writing or modifying any code that uses a library, consult Context7 MCP to verify the API. Training data is outdated for Vite 8 beta, Oxfmt beta, and recent Hono/Zod changes.

---

## Task 1: Project Initialization

**Files:**

- Create: `package.json`
- Create: `tsconfig.json`
- Create: `vite.config.ts`
- Create: `.oxlintrc.json`
- Create: `.oxfmtrc.json`
- Create: `.env.example`

**Step 1: Create package.json**

```json
{
  "name": "github-stats-cards",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "start": "node dist/index.js",
    "lint": "oxlint",
    "lint:fix": "oxlint --fix",
    "format": "oxfmt --write .",
    "format:check": "oxfmt --check .",
    "typecheck": "tsc --noEmit",
    "check": "pnpm format:check && pnpm lint && pnpm typecheck",
    "test": "vitest run",
    "test:watch": "vitest",
    "docker:build": "docker compose build",
    "docker:up": "docker compose up -d",
    "docker:down": "docker compose down",
    "docker:logs": "docker compose logs -f app"
  },
  "dependencies": {
    "hono": "^4",
    "@hono/node-server": "^1",
    "zod": "^3",
    "lru-cache": "^11",
    "ioredis": "^5"
  },
  "devDependencies": {
    "typescript": "^5.8",
    "vite": "^8.0.0-beta",
    "@hono/vite-build": "^1",
    "@hono/vite-dev-server": "^0",
    "vitest": "^3",
    "@types/node": "^24",
    "oxlint": "^0.16"
  },
  "volta": {
    "node": "24.13.1",
    "pnpm": "10.5.2"
  }
}
```

> **Context7:** Check `@hono/vite-build` and `@hono/vite-dev-server` for latest version ranges. Check `oxlint` for current version. Check `vitest` compatibility with Vite 8 beta.

**Step 2: Create tsconfig.json**

```json
{
  "compilerOptions": {
    "strict": true,
    "target": "ES2024",
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true,
    "outDir": "dist",
    "rootDir": "src",
    "types": ["node"]
  },
  "include": ["src/**/*.ts"],
  "exclude": ["node_modules", "dist"]
}
```

**Step 3: Create vite.config.ts**

```typescript
import build from '@hono/vite-build/node';
import devServer from '@hono/vite-dev-server';
import { defineConfig } from 'vite';

export default defineConfig(({ mode }) => {
  if (mode === 'client') {
    throw new Error('This is a server-only project');
  }

  return {
    plugins: [
      devServer({
        entry: 'src/index.ts',
      }),
      build({
        entry: 'src/index.ts',
      }),
    ],
  };
});
```

> **Context7:** Verify `@hono/vite-build/node` plugin options (entry, output) and `@hono/vite-dev-server` options (entry, nodeAdapter) for Vite 8. The API may have changed from Vite 7.

**Step 4: Create .oxlintrc.json**

```json
{
  "$schema": "https://raw.githubusercontent.com/nicolo-ribaudo/oxc-config-schema/refs/heads/main/oxlintrc.json",
  "categories": {
    "correctness": "error",
    "suspicious": "warn"
  },
  "ignorePatterns": ["dist/", "node_modules/"]
}
```

> **Context7:** Check Oxlint config schema for current category names and config format.

**Step 5: Create .oxfmtrc.json**

```json
{
  "printWidth": 100,
  "singleQuote": true,
  "trailingComma": "all",
  "semi": true
}
```

> **Context7:** Check Oxfmt config schema for supported options and any beta changes.

**Step 6: Create .env.example**

```
# GitHub Personal Access Tokens (at least PAT_1 required)
PAT_1=ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
# PAT_2=ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx

# Server
PORT=3000

# Redis (optional — falls back to in-memory LRU if not set)
# REDIS_URL=redis://localhost:6379

# Cache
CACHE_TTL=14400

# Logging
LOG_LEVEL=info
```

**Step 7: Install dependencies and verify**

Run: `pnpm install`
Expected: Clean install, lockfile generated, no errors

Run: `pnpm typecheck`
Expected: May fail (no src files yet) — that's fine

**Step 8: Commit**

```bash
git add package.json pnpm-lock.yaml tsconfig.json vite.config.ts .oxlintrc.json .oxfmtrc.json .env.example
git commit -m "chore: initialize project with deps and configs"
```

---

## Task 2: Utility — HTML Sanitization

**Files:**

- Create: `src/utils/sanitize.ts`
- Create: `src/utils/sanitize.test.ts`

**Step 1: Write the failing test**

```typescript
import { describe, expect, it } from 'vitest';
import { encodeHTML } from './sanitize.js';

describe('encodeHTML', () => {
  it('escapes ampersands', () => {
    expect(encodeHTML('a & b')).toBe('a &amp; b');
  });

  it('escapes angle brackets', () => {
    expect(encodeHTML('<script>')).toBe('&lt;script&gt;');
  });

  it('escapes double quotes', () => {
    expect(encodeHTML('"hello"')).toBe('&quot;hello&quot;');
  });

  it('escapes single quotes', () => {
    expect(encodeHTML("it's")).toBe('it&#39;s');
  });

  it('handles empty string', () => {
    expect(encodeHTML('')).toBe('');
  });

  it('handles string with no special chars', () => {
    expect(encodeHTML('hello world')).toBe('hello world');
  });

  it('escapes multiple special chars in one string', () => {
    expect(encodeHTML('<a href="x">&</a>')).toBe('&lt;a href=&quot;x&quot;&gt;&amp;&lt;/a&gt;');
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm vitest run src/utils/sanitize.test.ts`
Expected: FAIL — module not found

**Step 3: Write minimal implementation**

```typescript
const ESCAPE_MAP: Record<string, string> = {
  '&': '&amp;',
  '<': '&lt;',
  '>': '&gt;',
  '"': '&quot;',
  "'": '&#39;',
};

const ESCAPE_RE = /[&<>"']/g;

export function encodeHTML(str: string): string {
  return str.replace(ESCAPE_RE, (char) => ESCAPE_MAP[char]!);
}
```

**Step 4: Run test to verify it passes**

Run: `pnpm vitest run src/utils/sanitize.test.ts`
Expected: All 7 tests PASS

**Step 5: Commit**

```bash
git add src/utils/sanitize.ts src/utils/sanitize.test.ts
git commit -m "feat: add HTML/SVG entity escaping utility"
```

---

## Task 3: Utility — PAT Token Pool

**Files:**

- Create: `src/utils/pat-pool.ts`
- Create: `src/utils/pat-pool.test.ts`

**Step 1: Write the failing test**

```typescript
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createPatPool } from './pat-pool.js';

describe('createPatPool', () => {
  it('throws if no tokens provided', () => {
    expect(() => createPatPool([])).toThrow('At least one PAT is required');
  });

  it('returns tokens in round-robin order', () => {
    const pool = createPatPool(['a', 'b', 'c']);
    expect(pool.getNextToken()).toBe('a');
    expect(pool.getNextToken()).toBe('b');
    expect(pool.getNextToken()).toBe('c');
    expect(pool.getNextToken()).toBe('a');
  });

  it('works with a single token', () => {
    const pool = createPatPool(['only']);
    expect(pool.getNextToken()).toBe('only');
    expect(pool.getNextToken()).toBe('only');
  });

  it('skips exhausted tokens', () => {
    const pool = createPatPool(['a', 'b', 'c']);
    pool.markExhausted('b');
    expect(pool.getNextToken()).toBe('a');
    expect(pool.getNextToken()).toBe('c');
    expect(pool.getNextToken()).toBe('a');
  });

  it('throws when all tokens are exhausted', () => {
    const pool = createPatPool(['a']);
    pool.markExhausted('a');
    expect(() => pool.getNextToken()).toThrow('All PAT tokens are rate-limited');
  });

  it('resets exhausted tokens after cooldown', () => {
    vi.useFakeTimers();
    const pool = createPatPool(['a']);
    pool.markExhausted('a');
    vi.advanceTimersByTime(60 * 60 * 1000); // 1 hour
    expect(pool.getNextToken()).toBe('a');
    vi.useRealTimers();
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm vitest run src/utils/pat-pool.test.ts`
Expected: FAIL — module not found

**Step 3: Write minimal implementation**

```typescript
interface PatPool {
  getNextToken(): string;
  markExhausted(token: string): void;
}

const COOLDOWN_MS = 60 * 60 * 1000; // 1 hour

export function createPatPool(tokens: string[]): PatPool {
  if (tokens.length === 0) {
    throw new Error('At least one PAT is required');
  }

  let index = 0;
  const exhaustedUntil = new Map<string, number>();

  function isAvailable(token: string): boolean {
    const until = exhaustedUntil.get(token);
    if (until === undefined) return true;
    if (Date.now() >= until) {
      exhaustedUntil.delete(token);
      return true;
    }
    return false;
  }

  return {
    getNextToken(): string {
      const startIndex = index;
      do {
        const token = tokens[index % tokens.length]!;
        index = (index + 1) % tokens.length;
        if (isAvailable(token)) return token;
      } while (index !== startIndex);

      throw new Error('All PAT tokens are rate-limited');
    },

    markExhausted(token: string): void {
      exhaustedUntil.set(token, Date.now() + COOLDOWN_MS);
    },
  };
}
```

**Step 4: Run test to verify it passes**

Run: `pnpm vitest run src/utils/pat-pool.test.ts`
Expected: All 6 tests PASS

**Step 5: Commit**

```bash
git add src/utils/pat-pool.ts src/utils/pat-pool.test.ts
git commit -m "feat: add PAT token pool with round-robin rotation"
```

---

## Task 4: Config & Types

**Files:**

- Create: `src/types.ts`
- Create: `src/config.ts`
- Create: `src/config.test.ts`

**Step 1: Create shared types**

```typescript
// src/types.ts

export interface CardOptions {
  theme: string;
  hide: string[];
  showIcons: boolean;
  hideBorder: boolean;
  hideTitle: boolean;
  bgColor?: string;
  titleColor?: string;
  textColor?: string;
  iconColor?: string;
  borderColor?: string;
  cacheSeconds: number;
  locale: string;
}

export interface GitHubStats {
  username: string;
  name: string;
  totalStars: number;
  totalForks: number;
  totalCommits: number;
  totalPRs: number;
  totalIssues: number;
  contributedTo: number;
}

export interface StreakData {
  username: string;
  totalContributions: number;
  currentStreak: number;
  longestStreak: number;
  currentStreakStart: string;
  currentStreakEnd: string;
  longestStreakStart: string;
  longestStreakEnd: string;
}

export interface LanguageData {
  name: string;
  percentage: number;
  color: string;
  size: number;
}

export interface TopLangsData {
  username: string;
  languages: LanguageData[];
}
```

**Step 2: Write the failing config test**

```typescript
import { describe, expect, it, vi } from 'vitest';

describe('loadConfig', () => {
  it('loads config with required PAT_1', async () => {
    vi.stubEnv('PAT_1', 'ghp_test123');
    vi.stubEnv('PORT', '4000');

    const { loadConfig } = await import('./config.js');
    const config = loadConfig();

    expect(config.pats).toEqual(['ghp_test123']);
    expect(config.port).toBe(4000);
    expect(config.redisUrl).toBeUndefined();
    expect(config.cacheTtl).toBe(14400);
    expect(config.logLevel).toBe('info');

    vi.unstubAllEnvs();
  });

  it('collects multiple PATs', async () => {
    vi.stubEnv('PAT_1', 'a');
    vi.stubEnv('PAT_2', 'b');
    vi.stubEnv('PAT_3', 'c');

    const { loadConfig } = await import('./config.js');
    const config = loadConfig();

    expect(config.pats).toEqual(['a', 'b', 'c']);

    vi.unstubAllEnvs();
  });

  it('throws if PAT_1 is missing', async () => {
    vi.stubEnv('PAT_1', '');

    const { loadConfig } = await import('./config.js');
    expect(() => loadConfig()).toThrow();

    vi.unstubAllEnvs();
  });
});
```

> **Note:** These tests use `vi.stubEnv` which requires vitest. The dynamic import is needed to re-evaluate the module with different env vars.

**Step 3: Run test to verify it fails**

Run: `pnpm vitest run src/config.test.ts`
Expected: FAIL — module not found

**Step 4: Write config implementation**

```typescript
// src/config.ts
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
```

> **Context7:** Verify `z.coerce.number()` behavior and `z.enum()` defaults with current Zod.

**Step 5: Run test to verify it passes**

Run: `pnpm vitest run src/config.test.ts`
Expected: All 3 tests PASS

**Step 6: Commit**

```bash
git add src/types.ts src/config.ts src/config.test.ts
git commit -m "feat: add config loader with Zod validation and shared types"
```

---

## Task 5: Query Parameter Validation

**Files:**

- Create: `src/utils/query-params.ts`
- Create: `src/utils/query-params.test.ts`

**Step 1: Write the failing test**

```typescript
import { describe, expect, it } from 'vitest';
import { parseCardOptions } from './query-params.js';

describe('parseCardOptions', () => {
  it('returns defaults for empty query', () => {
    const result = parseCardOptions({});
    expect(result).toEqual({
      theme: 'default',
      hide: [],
      showIcons: true,
      hideBorder: false,
      hideTitle: false,
      cacheSeconds: 14400,
      locale: 'en',
    });
  });

  it('parses theme', () => {
    const result = parseCardOptions({ theme: 'dark' });
    expect(result.theme).toBe('dark');
  });

  it('parses hide as comma-separated list', () => {
    const result = parseCardOptions({ hide: 'stars,forks' });
    expect(result.hide).toEqual(['stars', 'forks']);
  });

  it('parses boolean flags', () => {
    const result = parseCardOptions({
      show_icons: 'false',
      hide_border: 'true',
      hide_title: 'true',
    });
    expect(result.showIcons).toBe(false);
    expect(result.hideBorder).toBe(true);
    expect(result.hideTitle).toBe(true);
  });

  it('parses color overrides', () => {
    const result = parseCardOptions({ bg_color: 'ff0000', text_color: '00ff00' });
    expect(result.bgColor).toBe('ff0000');
    expect(result.textColor).toBe('00ff00');
  });

  it('enforces minimum cache_seconds of 1800', () => {
    const result = parseCardOptions({ cache_seconds: '100' });
    expect(result.cacheSeconds).toBe(1800);
  });

  it('returns undefined for parsing failure', () => {
    // cache_seconds is NaN-like
    const result = parseCardOptions({ cache_seconds: 'not-a-number' });
    expect(result.cacheSeconds).toBe(14400); // falls back to default
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm vitest run src/utils/query-params.test.ts`
Expected: FAIL — module not found

**Step 3: Write implementation**

```typescript
// src/utils/query-params.ts
import { z } from 'zod';
import type { CardOptions } from '../types.js';

const booleanString = z
  .enum(['true', 'false', '1', '0', ''])
  .transform((v) => v === 'true' || v === '1')
  .default('true');

const booleanStringFalse = z
  .enum(['true', 'false', '1', '0', ''])
  .transform((v) => v === 'true' || v === '1')
  .default('false');

const querySchema = z.object({
  theme: z.string().default('default'),
  hide: z
    .string()
    .default('')
    .transform((v) => (v ? v.split(',').map((s) => s.trim()) : [])),
  show_icons: booleanString,
  hide_border: booleanStringFalse,
  hide_title: booleanStringFalse,
  bg_color: z.string().optional(),
  title_color: z.string().optional(),
  text_color: z.string().optional(),
  icon_color: z.string().optional(),
  border_color: z.string().optional(),
  cache_seconds: z.coerce
    .number()
    .catch(14400)
    .transform((v) => Math.max(v, 1800))
    .default(14400),
  locale: z.string().default('en'),
});

export function parseCardOptions(query: Record<string, string | undefined>): CardOptions {
  const parsed = querySchema.parse(query);
  return {
    theme: parsed.theme,
    hide: parsed.hide,
    showIcons: parsed.show_icons,
    hideBorder: parsed.hide_border,
    hideTitle: parsed.hide_title,
    bgColor: parsed.bg_color,
    titleColor: parsed.title_color,
    textColor: parsed.text_color,
    iconColor: parsed.icon_color,
    borderColor: parsed.border_color,
    cacheSeconds: parsed.cache_seconds,
    locale: parsed.locale,
  };
}
```

> **Context7:** Verify Zod `.catch()`, `.transform()`, `.default()` chaining order. Verify `z.coerce.number()` behavior with non-numeric strings.

**Step 4: Run test to verify it passes**

Run: `pnpm vitest run src/utils/query-params.test.ts`
Expected: All 7 tests PASS

**Step 5: Commit**

```bash
git add src/utils/query-params.ts src/utils/query-params.test.ts
git commit -m "feat: add query parameter validation with Zod"
```

---

## Task 6: Theme System

**Files:**

- Create: `src/themes/themes.ts`
- Create: `src/themes/index.ts`
- Create: `src/themes/themes.test.ts`

**Step 1: Write the failing test**

```typescript
import { describe, expect, it } from 'vitest';
import { getTheme, THEME_NAMES } from './index.js';

describe('theme system', () => {
  it('returns default theme', () => {
    const theme = getTheme('default');
    expect(theme.name).toBe('default');
    expect(theme.background).toBeDefined();
    expect(theme.text).toBeDefined();
  });

  it('returns dark theme', () => {
    const theme = getTheme('dark');
    expect(theme.name).toBe('dark');
  });

  it('returns dracula theme', () => {
    const theme = getTheme('dracula');
    expect(theme.name).toBe('dracula');
  });

  it('falls back to default for unknown theme', () => {
    const theme = getTheme('nonexistent');
    expect(theme.name).toBe('default');
  });

  it('lists all available theme names', () => {
    expect(THEME_NAMES).toContain('default');
    expect(THEME_NAMES).toContain('dark');
    expect(THEME_NAMES).toContain('dracula');
    expect(THEME_NAMES.length).toBe(3);
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm vitest run src/themes/themes.test.ts`
Expected: FAIL — module not found

**Step 3: Write theme definitions**

```typescript
// src/themes/themes.ts

export interface Theme {
  name: string;
  background: string;
  backgroundBlur: string;
  border: string;
  title: string;
  text: string;
  muted: string;
  icon: string;
  ring: string;
}

export const defaultTheme: Theme = {
  name: 'default',
  background: 'rgba(255, 255, 255, 0.7)',
  backgroundBlur: 'rgba(255, 255, 255, 0.4)',
  border: 'rgba(0, 0, 0, 0.1)',
  title: '#1f2937',
  text: '#374151',
  muted: '#6b7280',
  icon: '#3b82f6',
  ring: '#3b82f6',
};

export const darkTheme: Theme = {
  name: 'dark',
  background: 'rgba(13, 17, 23, 0.8)',
  backgroundBlur: 'rgba(13, 17, 23, 0.5)',
  border: 'rgba(255, 255, 255, 0.1)',
  title: '#e6edf3',
  text: '#c9d1d9',
  muted: '#8b949e',
  icon: '#58a6ff',
  ring: '#58a6ff',
};

export const draculaTheme: Theme = {
  name: 'dracula',
  background: 'rgba(40, 42, 54, 0.8)',
  backgroundBlur: 'rgba(40, 42, 54, 0.5)',
  border: 'rgba(98, 114, 164, 0.3)',
  title: '#f8f8f2',
  text: '#f8f8f2',
  muted: '#6272a4',
  icon: '#bd93f9',
  ring: '#bd93f9',
};
```

**Step 4: Write theme registry**

```typescript
// src/themes/index.ts
import type { Theme } from './themes.js';
import { darkTheme, defaultTheme, draculaTheme } from './themes.js';

export type { Theme } from './themes.js';

const themes: Record<string, Theme> = {
  default: defaultTheme,
  dark: darkTheme,
  dracula: draculaTheme,
};

export const THEME_NAMES: string[] = Object.keys(themes);

export function getTheme(name: string): Theme {
  return themes[name] ?? defaultTheme;
}
```

**Step 5: Run test to verify it passes**

Run: `pnpm vitest run src/themes/themes.test.ts`
Expected: All 5 tests PASS

**Step 6: Commit**

```bash
git add src/themes/
git commit -m "feat: add theme system with default, dark, and dracula themes"
```

---

## Task 7: Cache Layer

**Files:**

- Create: `src/cache/index.ts`
- Create: `src/cache/memory.ts`
- Create: `src/cache/redis.ts`
- Create: `src/cache/memory.test.ts`

**Step 1: Write the failing test for memory cache**

```typescript
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createMemoryCache } from './memory.js';

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

  it('respects TTL', async () => {
    vi.useFakeTimers();
    const cache = createMemoryCache();
    await cache.set('key', 'value', 1); // 1 second TTL
    vi.advanceTimersByTime(2000);
    expect(await cache.get('key')).toBeNull();
    vi.useRealTimers();
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm vitest run src/cache/memory.test.ts`
Expected: FAIL — module not found

**Step 3: Write cache interface and memory implementation**

```typescript
// src/cache/index.ts

export interface Cache {
  get(key: string): Promise<string | null>;
  set(key: string, value: string, ttlSeconds: number): Promise<void>;
}
```

```typescript
// src/cache/memory.ts
import { LRUCache } from 'lru-cache';
import type { Cache } from './index.js';

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
```

> **Context7:** Verify `lru-cache` v11 constructor options. Confirm `ttl` is in ms and passed per-entry via `.set()` options.

**Step 4: Run test to verify it passes**

Run: `pnpm vitest run src/cache/memory.test.ts`
Expected: All 3 tests PASS

**Step 5: Write Redis cache implementation**

```typescript
// src/cache/redis.ts
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
```

> **Context7:** Verify `ioredis` constructor accepts URL string directly. Verify `setex` signature.

**Step 6: Update cache/index.ts with factory**

```typescript
// src/cache/index.ts

export interface Cache {
  get(key: string): Promise<string | null>;
  set(key: string, value: string, ttlSeconds: number): Promise<void>;
}

export function createCache(redisUrl?: string): Cache {
  if (redisUrl) {
    // Dynamic import to avoid loading ioredis when not needed
    const { createRedisCache } = require('./redis.js');
    return createRedisCache(redisUrl);
  }
  const { createMemoryCache } = require('./memory.js');
  return createMemoryCache();
}

export { createMemoryCache } from './memory.js';
export { createRedisCache } from './redis.js';
```

> **Note:** The factory uses `require()` for conditional loading. In the final version, consider using dynamic `import()` instead. Adjust based on Vite 8 SSR bundling behavior (check Context7).

Actually, use static imports since this is a server-only project and tree-shaking doesn't matter:

```typescript
// src/cache/index.ts

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
```

**Step 7: Commit**

```bash
git add src/cache/
git commit -m "feat: add cache layer with LRU memory and Redis backends"
```

---

## Task 8: GitHub Fetcher

**Files:**

- Create: `src/fetchers/types.ts`
- Create: `src/fetchers/github.ts`
- Create: `src/fetchers/github.test.ts`

**Step 1: Write GitHub API response types**

```typescript
// src/fetchers/types.ts

export interface GitHubGraphQLResponse {
  data?: {
    user: GitHubUser | null;
  };
  errors?: Array<{ message: string }>;
}

export interface GitHubUser {
  name: string | null;
  login: string;
  contributionsCollection: {
    totalCommitContributions: number;
    restrictedContributionsCount: number;
    contributionCalendar: {
      totalContributions: number;
      weeks: Array<{
        contributionDays: Array<{
          contributionCount: number;
          date: string;
        }>;
      }>;
    };
  };
  repositories: {
    totalCount: number;
    nodes: Array<{
      stargazerCount: number;
      forkCount: number;
      languages: {
        edges: Array<{
          size: number;
          node: {
            name: string;
            color: string | null;
          };
        }>;
      };
    }>;
  };
  pullRequests: {
    totalCount: number;
  };
  issues: {
    totalCount: number;
  };
  followers: {
    totalCount: number;
  };
  repositoriesContributedTo: {
    totalCount: number;
  };
}
```

**Step 2: Write the failing test**

```typescript
import { afterEach, describe, expect, it, vi } from 'vitest';
import { fetchGitHubData } from './github.js';

const mockUser = {
  name: 'Test User',
  login: 'testuser',
  contributionsCollection: {
    totalCommitContributions: 100,
    restrictedContributionsCount: 10,
    contributionCalendar: {
      totalContributions: 500,
      weeks: [],
    },
  },
  repositories: {
    totalCount: 20,
    nodes: [
      {
        stargazerCount: 50,
        forkCount: 10,
        languages: {
          edges: [
            { size: 1000, node: { name: 'TypeScript', color: '#3178c6' } },
            { size: 500, node: { name: 'Python', color: '#3572A5' } },
          ],
        },
      },
    ],
  },
  pullRequests: { totalCount: 30 },
  issues: { totalCount: 15 },
  followers: { totalCount: 200 },
  repositoriesContributedTo: { totalCount: 5 },
};

describe('fetchGitHubData', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('fetches and transforms user stats', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({ data: { user: mockUser } }),
        headers: new Headers(),
      }),
    );

    const result = await fetchGitHubData('testuser', 'fake-token');

    expect(result.stats.username).toBe('testuser');
    expect(result.stats.totalStars).toBe(50);
    expect(result.stats.totalForks).toBe(10);
    expect(result.stats.totalCommits).toBe(110); // 100 + 10 restricted
    expect(result.stats.totalPRs).toBe(30);
    expect(result.stats.totalIssues).toBe(15);
    expect(result.stats.contributedTo).toBe(5);
  });

  it('aggregates language data', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({ data: { user: mockUser } }),
        headers: new Headers(),
      }),
    );

    const result = await fetchGitHubData('testuser', 'fake-token');

    expect(result.languages.languages).toHaveLength(2);
    expect(result.languages.languages[0]!.name).toBe('TypeScript');
    expect(result.languages.languages[0]!.percentage).toBeCloseTo(66.67, 1);
  });

  it('throws on user not found', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({ data: { user: null } }),
        headers: new Headers(),
      }),
    );

    await expect(fetchGitHubData('ghost', 'fake-token')).rejects.toThrow('User not found: ghost');
  });

  it('throws on API error', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: false,
        status: 403,
        statusText: 'Forbidden',
        headers: new Headers(),
      }),
    );

    await expect(fetchGitHubData('user', 'fake-token')).rejects.toThrow();
  });
});
```

**Step 3: Run test to verify it fails**

Run: `pnpm vitest run src/fetchers/github.test.ts`
Expected: FAIL — module not found

**Step 4: Write GitHub fetcher**

```typescript
// src/fetchers/github.ts
import type { GitHubGraphQLResponse, GitHubUser } from './types.js';
import type { GitHubStats, StreakData, TopLangsData } from '../types.js';

const GITHUB_GRAPHQL_URL = 'https://api.github.com/graphql';

const USER_QUERY = `
query($login: String!) {
  user(login: $login) {
    name
    login
    contributionsCollection {
      totalCommitContributions
      restrictedContributionsCount
      contributionCalendar {
        totalContributions
        weeks {
          contributionDays {
            contributionCount
            date
          }
        }
      }
    }
    repositories(first: 100, ownerAffiliations: OWNER, orderBy: {field: STARGAZERS, direction: DESC}) {
      totalCount
      nodes {
        stargazerCount
        forkCount
        languages(first: 10, orderBy: {field: SIZE, direction: DESC}) {
          edges {
            size
            node {
              name
              color
            }
          }
        }
      }
    }
    pullRequests(first: 1) {
      totalCount
    }
    issues(first: 1) {
      totalCount
    }
    followers {
      totalCount
    }
    repositoriesContributedTo(first: 1, contributionTypes: [COMMIT, ISSUE, PULL_REQUEST, REPOSITORY]) {
      totalCount
    }
  }
}
`;

export interface FetchResult {
  stats: GitHubStats;
  streak: StreakData;
  languages: TopLangsData;
}

export class GitHubApiError extends Error {
  constructor(
    message: string,
    public status?: number,
  ) {
    super(message);
    this.name = 'GitHubApiError';
  }
}

export async function fetchGitHubData(username: string, token: string): Promise<FetchResult> {
  const response = await fetch(GITHUB_GRAPHQL_URL, {
    method: 'POST',
    headers: {
      Authorization: `bearer ${token}`,
      'Content-Type': 'application/json',
      'User-Agent': 'github-stats-cards',
    },
    body: JSON.stringify({ query: USER_QUERY, variables: { login: username } }),
  });

  if (!response.ok) {
    throw new GitHubApiError(
      `GitHub API error: ${response.status} ${response.statusText}`,
      response.status,
    );
  }

  const json = (await response.json()) as GitHubGraphQLResponse;

  if (json.errors?.length) {
    throw new GitHubApiError(`GraphQL errors: ${json.errors.map((e) => e.message).join(', ')}`);
  }

  const user = json.data?.user;
  if (!user) {
    throw new GitHubApiError(`User not found: ${username}`, 404);
  }

  return {
    stats: extractStats(user, username),
    streak: extractStreak(user, username),
    languages: extractLanguages(user, username),
  };
}

function extractStats(user: GitHubUser, username: string): GitHubStats {
  const totalStars = user.repositories.nodes.reduce((sum, r) => sum + r.stargazerCount, 0);
  const totalForks = user.repositories.nodes.reduce((sum, r) => sum + r.forkCount, 0);

  return {
    username,
    name: user.name ?? user.login,
    totalStars,
    totalForks,
    totalCommits:
      user.contributionsCollection.totalCommitContributions +
      user.contributionsCollection.restrictedContributionsCount,
    totalPRs: user.pullRequests.totalCount,
    totalIssues: user.issues.totalCount,
    contributedTo: user.repositoriesContributedTo.totalCount,
  };
}

function extractStreak(user: GitHubUser, username: string): StreakData {
  const calendar = user.contributionsCollection.contributionCalendar;
  const days = calendar.weeks.flatMap((w) => w.contributionDays);

  let currentStreak = 0;
  let longestStreak = 0;
  let tempStreak = 0;
  let currentStreakStart = '';
  let currentStreakEnd = '';
  let longestStreakStart = '';
  let longestStreakEnd = '';
  let tempStart = '';

  for (const day of days) {
    if (day.contributionCount > 0) {
      if (tempStreak === 0) tempStart = day.date;
      tempStreak++;
      if (tempStreak > longestStreak) {
        longestStreak = tempStreak;
        longestStreakStart = tempStart;
        longestStreakEnd = day.date;
      }
    } else {
      tempStreak = 0;
    }
  }

  // Current streak: count backwards from today
  currentStreak = 0;
  for (let i = days.length - 1; i >= 0; i--) {
    if (days[i]!.contributionCount > 0) {
      currentStreak++;
      currentStreakEnd = currentStreakEnd || days[i]!.date;
      currentStreakStart = days[i]!.date;
    } else if (currentStreak > 0) {
      break;
    }
  }

  return {
    username,
    totalContributions: calendar.totalContributions,
    currentStreak,
    longestStreak,
    currentStreakStart,
    currentStreakEnd,
    longestStreakStart,
    longestStreakEnd,
  };
}

function extractLanguages(user: GitHubUser, username: string): TopLangsData {
  const langMap = new Map<string, { size: number; color: string }>();

  for (const repo of user.repositories.nodes) {
    for (const edge of repo.languages.edges) {
      const existing = langMap.get(edge.node.name);
      if (existing) {
        existing.size += edge.size;
      } else {
        langMap.set(edge.node.name, { size: edge.size, color: edge.node.color ?? '#858585' });
      }
    }
  }

  const totalSize = Array.from(langMap.values()).reduce((sum, l) => sum + l.size, 0);

  const languages = Array.from(langMap.entries())
    .map(([name, { size, color }]) => ({
      name,
      size,
      color,
      percentage: totalSize > 0 ? (size / totalSize) * 100 : 0,
    }))
    .sort((a, b) => b.size - a.size)
    .slice(0, 5);

  return { username, languages };
}
```

**Step 5: Run test to verify it passes**

Run: `pnpm vitest run src/fetchers/github.test.ts`
Expected: All 4 tests PASS

**Step 6: Commit**

```bash
git add src/fetchers/
git commit -m "feat: add GitHub GraphQL fetcher with stats, streak, and language extraction"
```

---

## Task 9: Base Card SVG Utilities

**Files:**

- Create: `src/cards/base-card.ts`
- Create: `src/cards/base-card.test.ts`

**Step 1: Write the failing test**

```typescript
import { describe, expect, it } from 'vitest';
import { createArcPath, createGlassFilter, formatNumber } from './base-card.js';

describe('formatNumber', () => {
  it('formats with default locale', () => {
    expect(formatNumber(1234, 'en')).toBe('1,234');
  });

  it('formats zero', () => {
    expect(formatNumber(0, 'en')).toBe('0');
  });

  it('formats large numbers', () => {
    expect(formatNumber(1234567, 'en')).toBe('1,234,567');
  });
});

describe('createArcPath', () => {
  it('returns a valid SVG path d attribute', () => {
    const d = createArcPath(50, 50, 40, 0, Math.PI);
    expect(d).toContain('M');
    expect(d).toContain('A');
  });

  it('handles full circle', () => {
    const d = createArcPath(50, 50, 40, 0, Math.PI * 2 - 0.001);
    expect(d).toContain('A');
  });
});

describe('createGlassFilter', () => {
  it('returns SVG filter definition', () => {
    const filter = createGlassFilter('glass-1');
    expect(filter).toContain('<filter');
    expect(filter).toContain('id="glass-1"');
    expect(filter).toContain('feGaussianBlur');
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm vitest run src/cards/base-card.test.ts`
Expected: FAIL — module not found

**Step 3: Write implementation**

```typescript
// src/cards/base-card.ts
import type { Theme } from '../themes/index.js';
import type { CardOptions } from '../types.js';
import { encodeHTML } from '../utils/sanitize.js';

export { encodeHTML } from '../utils/sanitize.js';

const FONT_FAMILY = "'Segoe UI', system-ui, -apple-system, sans-serif";

export function formatNumber(n: number, locale: string): string {
  return new Intl.NumberFormat(locale).format(n);
}

export function createArcPath(
  cx: number,
  cy: number,
  radius: number,
  startAngle: number,
  endAngle: number,
): string {
  const x1 = cx + radius * Math.cos(startAngle);
  const y1 = cy + radius * Math.sin(startAngle);
  const x2 = cx + radius * Math.cos(endAngle);
  const y2 = cy + radius * Math.sin(endAngle);
  const largeArc = endAngle - startAngle > Math.PI ? 1 : 0;

  return `M ${x1} ${y1} A ${radius} ${radius} 0 ${largeArc} 1 ${x2} ${y2}`;
}

export function createGlassFilter(id: string): string {
  return `
    <filter id="${id}" x="-10%" y="-10%" width="120%" height="120%">
      <feGaussianBlur in="SourceGraphic" stdDeviation="8" result="blur" />
      <feColorMatrix in="blur" type="saturate" values="1.2" result="saturated" />
      <feComposite in="SourceGraphic" in2="saturated" operator="over" />
    </filter>
  `;
}

export function createCardWrapper(
  width: number,
  height: number,
  content: string,
  theme: Theme,
  options: CardOptions,
): string {
  const bg = options.bgColor ? `#${options.bgColor}` : theme.background;
  const borderColor = options.borderColor ? `#${options.borderColor}` : theme.border;
  const showBorder = !options.hideBorder;
  const radius = 8;

  return `
<svg width="${width}" height="${height}" viewBox="0 0 ${width} ${height}"
     xmlns="http://www.w3.org/2000/svg">
  <defs>
    ${createGlassFilter('glass')}
    <clipPath id="card-clip">
      <rect width="${width}" height="${height}" rx="${radius}" ry="${radius}" />
    </clipPath>
  </defs>

  <style>
    * { font-family: ${FONT_FAMILY}; }
    @keyframes fadeIn {
      from { opacity: 0; transform: translateY(5px); }
      to { opacity: 1; transform: translateY(0); }
    }
    .fade-in { animation: fadeIn 0.6s ease-out forwards; }
    .title { font-size: 14px; font-weight: 600; fill: ${options.titleColor ? `#${options.titleColor}` : theme.title}; }
    .stat-label { font-size: 12px; fill: ${options.textColor ? `#${options.textColor}` : theme.muted}; }
    .stat-value { font-size: 14px; font-weight: 700; fill: ${options.textColor ? `#${options.textColor}` : theme.text}; }
    .muted { font-size: 11px; fill: ${theme.muted}; }
  </style>

  <g clip-path="url(#card-clip)">
    <!-- Glass backdrop -->
    <rect width="${width}" height="${height}" rx="${radius}" ry="${radius}"
          fill="${theme.backgroundBlur}" filter="url(#glass)" />
    <!-- Glass overlay -->
    <rect width="${width}" height="${height}" rx="${radius}" ry="${radius}"
          fill="${bg}" />
    ${showBorder ? `<rect width="${width}" height="${height}" rx="${radius}" ry="${radius}" fill="none" stroke="${borderColor}" stroke-width="1" />` : ''}

    ${content}
  </g>
</svg>`.trim();
}

export function createRingChart(
  cx: number,
  cy: number,
  radius: number,
  percentage: number,
  color: string,
  bgColor: string,
  label: string,
): string {
  const circumference = 2 * Math.PI * radius;
  const offset = circumference * (1 - percentage / 100);

  return `
    <circle cx="${cx}" cy="${cy}" r="${radius}"
            fill="none" stroke="${bgColor}" stroke-width="6" opacity="0.2" />
    <circle cx="${cx}" cy="${cy}" r="${radius}"
            fill="none" stroke="${color}" stroke-width="6"
            stroke-dasharray="${circumference}"
            stroke-dashoffset="${offset}"
            stroke-linecap="round"
            transform="rotate(-90 ${cx} ${cy})"
            class="fade-in" />
    <text x="${cx}" y="${cy}" text-anchor="middle" dominant-baseline="central"
          font-size="18" font-weight="700" fill="${color}">
      ${encodeHTML(label)}
    </text>
  `;
}

export function createDonutSegment(
  cx: number,
  cy: number,
  radius: number,
  startAngle: number,
  endAngle: number,
  color: string,
): string {
  const innerRadius = radius * 0.6;
  const d = createDonutArcPath(cx, cy, radius, innerRadius, startAngle, endAngle);
  return `<path d="${d}" fill="${color}" class="fade-in" />`;
}

function createDonutArcPath(
  cx: number,
  cy: number,
  outerR: number,
  innerR: number,
  startAngle: number,
  endAngle: number,
): string {
  const startOuter = polarToCartesian(cx, cy, outerR, startAngle);
  const endOuter = polarToCartesian(cx, cy, outerR, endAngle);
  const startInner = polarToCartesian(cx, cy, innerR, endAngle);
  const endInner = polarToCartesian(cx, cy, innerR, startAngle);
  const largeArc = endAngle - startAngle > Math.PI ? 1 : 0;

  return [
    `M ${startOuter.x} ${startOuter.y}`,
    `A ${outerR} ${outerR} 0 ${largeArc} 1 ${endOuter.x} ${endOuter.y}`,
    `L ${startInner.x} ${startInner.y}`,
    `A ${innerR} ${innerR} 0 ${largeArc} 0 ${endInner.x} ${endInner.y}`,
    'Z',
  ].join(' ');
}

function polarToCartesian(
  cx: number,
  cy: number,
  radius: number,
  angle: number,
): { x: number; y: number } {
  return {
    x: cx + radius * Math.cos(angle - Math.PI / 2),
    y: cy + radius * Math.sin(angle - Math.PI / 2),
  };
}
```

**Step 4: Run test to verify it passes**

Run: `pnpm vitest run src/cards/base-card.test.ts`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src/cards/base-card.ts src/cards/base-card.test.ts
git commit -m "feat: add base card SVG utilities (glass filter, arcs, ring chart, donut)"
```

---

## Task 10: Stats Card

**Files:**

- Create: `src/cards/stats-card.ts`
- Create: `src/cards/stats-card.test.ts`

**Step 1: Write the failing test**

```typescript
import { describe, expect, it } from 'vitest';
import { renderStatsCard } from './stats-card.js';
import type { GitHubStats, CardOptions } from '../types.js';

const mockStats: GitHubStats = {
  username: 'testuser',
  name: 'Test User',
  totalStars: 1234,
  totalForks: 567,
  totalCommits: 2345,
  totalPRs: 89,
  totalIssues: 45,
  contributedTo: 12,
};

const defaultOptions: CardOptions = {
  theme: 'default',
  hide: [],
  showIcons: true,
  hideBorder: false,
  hideTitle: false,
  cacheSeconds: 14400,
  locale: 'en',
};

describe('renderStatsCard', () => {
  it('returns valid SVG', () => {
    const svg = renderStatsCard(mockStats, defaultOptions);
    expect(svg).toContain('<svg');
    expect(svg).toContain('</svg>');
    expect(svg).toContain('495');
    expect(svg).toContain('195');
  });

  it('includes username in title', () => {
    const svg = renderStatsCard(mockStats, defaultOptions);
    expect(svg).toContain('Test User');
  });

  it('includes stat values', () => {
    const svg = renderStatsCard(mockStats, defaultOptions);
    expect(svg).toContain('1,234');
    expect(svg).toContain('2,345');
  });

  it('hides title when hideTitle is true', () => {
    const svg = renderStatsCard(mockStats, { ...defaultOptions, hideTitle: true });
    expect(svg).not.toContain('Test User');
  });

  it('hides specified stats', () => {
    const svg = renderStatsCard(mockStats, { ...defaultOptions, hide: ['stars'] });
    expect(svg).not.toContain('1,234');
  });

  it('includes grade ring', () => {
    const svg = renderStatsCard(mockStats, defaultOptions);
    // Should have a circle element for the ring chart
    expect(svg).toContain('<circle');
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm vitest run src/cards/stats-card.test.ts`
Expected: FAIL — module not found

**Step 3: Write implementation**

```typescript
// src/cards/stats-card.ts
import type { CardOptions, GitHubStats } from '../types.js';
import { getTheme } from '../themes/index.js';
import { createCardWrapper, createRingChart, encodeHTML, formatNumber } from './base-card.js';

const CARD_WIDTH = 495;
const CARD_HEIGHT = 195;

interface StatRow {
  key: string;
  label: string;
  value: number;
  icon: string;
}

function calculateGrade(stats: GitHubStats): { grade: string; percentage: number } {
  const score =
    stats.totalCommits * 1 +
    stats.totalPRs * 3 +
    stats.totalIssues * 2 +
    stats.totalStars * 1 +
    stats.totalForks * 0.5 +
    stats.contributedTo * 5;

  if (score >= 5000) return { grade: 'A+', percentage: 100 };
  if (score >= 2500) return { grade: 'A', percentage: 85 };
  if (score >= 1000) return { grade: 'B+', percentage: 70 };
  if (score >= 500) return { grade: 'B', percentage: 55 };
  return { grade: 'C', percentage: 40 };
}

export function renderStatsCard(stats: GitHubStats, options: CardOptions): string {
  const theme = getTheme(options.theme);
  const iconColor = options.iconColor ? `#${options.iconColor}` : theme.icon;
  const { grade, percentage } = calculateGrade(stats);

  const allStats: StatRow[] = [
    { key: 'stars', label: 'Total Stars', value: stats.totalStars, icon: '\u2605' },
    { key: 'forks', label: 'Total Forks', value: stats.totalForks, icon: '\u0192' },
    { key: 'commits', label: 'Total Commits', value: stats.totalCommits, icon: '\u25CB' },
    { key: 'prs', label: 'Total PRs', value: stats.totalPRs, icon: '\u25B3' },
    { key: 'issues', label: 'Total Issues', value: stats.totalIssues, icon: '!' },
    { key: 'contribs', label: 'Contributed To', value: stats.contributedTo, icon: '\u25C7' },
  ];

  const visibleStats = allStats.filter((s) => !options.hide.includes(s.key));

  const titleSection = options.hideTitle
    ? ''
    : `<text x="25" y="30" class="title">${encodeHTML(stats.name)}&apos;s GitHub Stats</text>`;

  const titleOffset = options.hideTitle ? 0 : 10;
  const ringCx = 75;
  const ringCy = 105 + titleOffset;
  const ringRadius = 40;

  const ringSection = createRingChart(
    ringCx,
    ringCy,
    ringRadius,
    percentage,
    theme.ring,
    theme.muted,
    grade,
  );

  const statsStartX = 160;
  const statsStartY = 55 + titleOffset;
  const rowHeight = 22;

  const statsSection = visibleStats
    .map((stat, i) => {
      const y = statsStartY + i * rowHeight;
      const iconPart = options.showIcons
        ? `<text x="${statsStartX}" y="${y}" font-size="14" fill="${iconColor}">${stat.icon}</text>`
        : '';
      const labelX = options.showIcons ? statsStartX + 22 : statsStartX;
      return `
        <g class="fade-in" style="animation-delay: ${i * 0.1}s;">
          ${iconPart}
          <text x="${labelX}" y="${y}" class="stat-label">${encodeHTML(stat.label)}</text>
          <text x="${CARD_WIDTH - 25}" y="${y}" class="stat-value" text-anchor="end">${formatNumber(stat.value, options.locale)}</text>
        </g>
      `;
    })
    .join('');

  const content = `
    ${titleSection}
    ${ringSection}
    ${statsSection}
  `;

  return createCardWrapper(CARD_WIDTH, CARD_HEIGHT, content, theme, options);
}
```

**Step 4: Run test to verify it passes**

Run: `pnpm vitest run src/cards/stats-card.test.ts`
Expected: All 6 tests PASS

**Step 5: Commit**

```bash
git add src/cards/stats-card.ts src/cards/stats-card.test.ts
git commit -m "feat: add stats card with ring chart grade and glassmorphism"
```

---

## Task 11: Streak Card

**Files:**

- Create: `src/cards/streak-card.ts`
- Create: `src/cards/streak-card.test.ts`

**Step 1: Write the failing test**

```typescript
import { describe, expect, it } from 'vitest';
import { renderStreakCard } from './streak-card.js';
import type { StreakData, CardOptions } from '../types.js';

const mockStreak: StreakData = {
  username: 'testuser',
  totalContributions: 2345,
  currentStreak: 47,
  longestStreak: 124,
  currentStreakStart: '2026-01-10',
  currentStreakEnd: '2026-02-26',
  longestStreakStart: '2024-05-01',
  longestStreakEnd: '2024-09-01',
};

const defaultOptions: CardOptions = {
  theme: 'default',
  hide: [],
  showIcons: true,
  hideBorder: false,
  hideTitle: false,
  cacheSeconds: 14400,
  locale: 'en',
};

describe('renderStreakCard', () => {
  it('returns valid SVG', () => {
    const svg = renderStreakCard(mockStreak, defaultOptions);
    expect(svg).toContain('<svg');
    expect(svg).toContain('</svg>');
  });

  it('includes streak numbers', () => {
    const svg = renderStreakCard(mockStreak, defaultOptions);
    expect(svg).toContain('47');
    expect(svg).toContain('124');
    expect(svg).toContain('2,345');
  });

  it('includes date ranges', () => {
    const svg = renderStreakCard(mockStreak, defaultOptions);
    expect(svg).toContain('Jan');
    expect(svg).toContain('Feb');
  });

  it('includes flame icon', () => {
    const svg = renderStreakCard(mockStreak, defaultOptions);
    // Should contain a path or text element for the flame
    expect(svg).toContain('path');
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm vitest run src/cards/streak-card.test.ts`
Expected: FAIL — module not found

**Step 3: Write implementation**

```typescript
// src/cards/streak-card.ts
import type { CardOptions, StreakData } from '../types.js';
import { getTheme } from '../themes/index.js';
import { createCardWrapper, encodeHTML, formatNumber } from './base-card.js';

const CARD_WIDTH = 495;
const CARD_HEIGHT = 195;

// Simplified flame SVG path
const FLAME_PATH =
  'M12 0C8.4 4.8 4 7.2 4 12c0 4.4 3.6 8 8 8s8-3.6 8-8c0-1.2-.4-2.4-.8-3.2-.4 1.6-1.6 2.4-2.8 2.4-2 0-3.2-1.6-3.2-3.2 0-1.6.8-3.2 1.6-4.4C13.2 2 12.4.8 12 0z';

function formatDateRange(start: string, end: string): string {
  if (!start || !end) return '';
  const startDate = new Date(start);
  const endDate = new Date(end);
  const fmt = new Intl.DateTimeFormat('en', { month: 'short', day: 'numeric' });
  return `${fmt.format(startDate)} - ${fmt.format(endDate)}`;
}

export function renderStreakCard(data: StreakData, options: CardOptions): string {
  const theme = getTheme(options.theme);
  const accentColor = options.iconColor ? `#${options.iconColor}` : theme.ring;
  const textColor = options.textColor ? `#${options.textColor}` : theme.text;
  const mutedColor = theme.muted;

  const titleSection = options.hideTitle
    ? ''
    : `<text x="${CARD_WIDTH / 2}" y="28" class="title" text-anchor="middle">Contribution Streak</text>`;

  const titleOffset = options.hideTitle ? -10 : 0;

  // Three columns: Total | Current (emphasized) | Longest
  const colWidth = CARD_WIDTH / 3;
  const col1X = colWidth / 2;
  const col2X = colWidth + colWidth / 2;
  const col3X = colWidth * 2 + colWidth / 2;

  const labelY = 62 + titleOffset;
  const valueY = 105 + titleOffset;
  const unitY = 125 + titleOffset;
  const dateY = 155 + titleOffset;

  const content = `
    ${titleSection}

    <!-- Total Contributions -->
    <g class="fade-in" style="animation-delay: 0s;">
      <text x="${col1X}" y="${labelY}" text-anchor="middle" class="stat-label">Total</text>
      <text x="${col1X}" y="${labelY + 16}" text-anchor="middle" class="stat-label">Contributions</text>
      <text x="${col1X}" y="${valueY}" text-anchor="middle"
            font-size="24" font-weight="700" fill="${textColor}">
        ${formatNumber(data.totalContributions, options.locale)}
      </text>
      <text x="${col1X}" y="${dateY}" text-anchor="middle" class="muted">
        ${encodeHTML(formatDateYear(data.currentStreakEnd))}
      </text>
    </g>

    <!-- Divider 1 -->
    <line x1="${colWidth}" y1="${55 + titleOffset}" x2="${colWidth}" y2="${165 + titleOffset}"
          stroke="${theme.border}" stroke-width="1" opacity="0.5" />

    <!-- Current Streak (emphasized) -->
    <g class="fade-in" style="animation-delay: 0.15s;">
      <g transform="translate(${col2X - 10}, ${labelY - 12}) scale(0.8)">
        <path d="${FLAME_PATH}" fill="${accentColor}" />
      </g>
      <text x="${col2X + 8}" y="${labelY}" text-anchor="middle" class="stat-label">Current Streak</text>
      <text x="${col2X}" y="${valueY + 4}" text-anchor="middle"
            font-size="32" font-weight="800" fill="${accentColor}">
        ${data.currentStreak}
      </text>
      <text x="${col2X}" y="${unitY + 6}" text-anchor="middle"
            font-size="13" fill="${mutedColor}">days</text>
      <text x="${col2X}" y="${dateY}" text-anchor="middle" class="muted">
        ${encodeHTML(formatDateRange(data.currentStreakStart, data.currentStreakEnd))}
      </text>
    </g>

    <!-- Divider 2 -->
    <line x1="${colWidth * 2}" y1="${55 + titleOffset}" x2="${colWidth * 2}" y2="${165 + titleOffset}"
          stroke="${theme.border}" stroke-width="1" opacity="0.5" />

    <!-- Longest Streak -->
    <g class="fade-in" style="animation-delay: 0.3s;">
      <text x="${col3X}" y="${labelY}" text-anchor="middle" class="stat-label">Longest</text>
      <text x="${col3X}" y="${labelY + 16}" text-anchor="middle" class="stat-label">Streak</text>
      <text x="${col3X}" y="${valueY}" text-anchor="middle"
            font-size="24" font-weight="700" fill="${textColor}">
        ${data.longestStreak}
      </text>
      <text x="${col3X}" y="${unitY}" text-anchor="middle"
            font-size="12" fill="${mutedColor}">days</text>
      <text x="${col3X}" y="${dateY}" text-anchor="middle" class="muted">
        ${encodeHTML(formatDateRange(data.longestStreakStart, data.longestStreakEnd))}
      </text>
    </g>
  `;

  return createCardWrapper(CARD_WIDTH, CARD_HEIGHT, content, theme, options);
}

function formatDateYear(dateStr: string): string {
  if (!dateStr) return '';
  const date = new Date(dateStr);
  return `Jan 1 - ${new Intl.DateTimeFormat('en', { month: 'short', day: 'numeric' }).format(date)}, ${date.getFullYear()}`;
}
```

**Step 4: Run test to verify it passes**

Run: `pnpm vitest run src/cards/streak-card.test.ts`
Expected: All 4 tests PASS

**Step 5: Commit**

```bash
git add src/cards/streak-card.ts src/cards/streak-card.test.ts
git commit -m "feat: add streak card with flame icon and three-column layout"
```

---

## Task 12: Top Languages Card

**Files:**

- Create: `src/cards/langs-card.ts`
- Create: `src/cards/langs-card.test.ts`

**Step 1: Write the failing test**

```typescript
import { describe, expect, it } from 'vitest';
import { renderLangsCard } from './langs-card.js';
import type { TopLangsData, CardOptions } from '../types.js';

const mockLangs: TopLangsData = {
  username: 'testuser',
  languages: [
    { name: 'TypeScript', percentage: 42.3, color: '#3178c6', size: 42300 },
    { name: 'Python', percentage: 28.1, color: '#3572A5', size: 28100 },
    { name: 'Rust', percentage: 15.4, color: '#dea584', size: 15400 },
    { name: 'Go', percentage: 8.7, color: '#00ADD8', size: 8700 },
    { name: 'Shell', percentage: 5.5, color: '#89e051', size: 5500 },
  ],
};

const defaultOptions: CardOptions = {
  theme: 'default',
  hide: [],
  showIcons: true,
  hideBorder: false,
  hideTitle: false,
  cacheSeconds: 14400,
  locale: 'en',
};

describe('renderLangsCard', () => {
  it('returns valid SVG', () => {
    const svg = renderLangsCard(mockLangs, defaultOptions);
    expect(svg).toContain('<svg');
    expect(svg).toContain('</svg>');
  });

  it('includes language names', () => {
    const svg = renderLangsCard(mockLangs, defaultOptions);
    expect(svg).toContain('TypeScript');
    expect(svg).toContain('Python');
    expect(svg).toContain('Rust');
  });

  it('includes percentages', () => {
    const svg = renderLangsCard(mockLangs, defaultOptions);
    expect(svg).toContain('42.3%');
    expect(svg).toContain('28.1%');
  });

  it('includes donut chart paths', () => {
    const svg = renderLangsCard(mockLangs, defaultOptions);
    // Donut segments are <path> elements
    expect(svg).toMatch(/<path[^>]*fill="#3178c6"/);
  });

  it('handles empty languages', () => {
    const emptyLangs: TopLangsData = { username: 'test', languages: [] };
    const svg = renderLangsCard(emptyLangs, defaultOptions);
    expect(svg).toContain('<svg');
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm vitest run src/cards/langs-card.test.ts`
Expected: FAIL — module not found

**Step 3: Write implementation**

```typescript
// src/cards/langs-card.ts
import type { CardOptions, TopLangsData } from '../types.js';
import { getTheme } from '../themes/index.js';
import { createCardWrapper, createDonutSegment, encodeHTML } from './base-card.js';

const CARD_WIDTH = 495;
const CARD_HEIGHT = 195;

export function renderLangsCard(data: TopLangsData, options: CardOptions): string {
  const theme = getTheme(options.theme);

  const titleSection = options.hideTitle
    ? ''
    : `<text x="25" y="30" class="title">Most Used Languages</text>`;

  const titleOffset = options.hideTitle ? -10 : 0;
  const donutCx = 90;
  const donutCy = 110 + titleOffset;
  const donutRadius = 55;

  // Build donut segments
  let currentAngle = 0;
  const donutSegments = data.languages
    .map((lang, i) => {
      const sweepAngle = (lang.percentage / 100) * Math.PI * 2;
      // Leave a tiny gap between segments
      const gap = 0.02;
      const startAngle = currentAngle + gap;
      const endAngle = currentAngle + sweepAngle - gap;
      currentAngle += sweepAngle;

      if (sweepAngle < 0.05) return ''; // Skip tiny segments

      return `<g class="fade-in" style="animation-delay: ${i * 0.1}s;">
        ${createDonutSegment(donutCx, donutCy, donutRadius, startAngle, endAngle, lang.color)}
      </g>`;
    })
    .join('');

  // Build legend
  const legendX = 185;
  const legendStartY = 60 + titleOffset;
  const legendRowHeight = 24;

  const legendItems = data.languages
    .map((lang, i) => {
      const y = legendStartY + i * legendRowHeight;
      return `
        <g class="fade-in" style="animation-delay: ${(i + data.languages.length) * 0.05}s;">
          <circle cx="${legendX}" cy="${y - 4}" r="5" fill="${lang.color}" />
          <text x="${legendX + 14}" y="${y}" class="stat-label">${encodeHTML(lang.name)}</text>
          <text x="${CARD_WIDTH - 25}" y="${y}" class="stat-value" text-anchor="end">${lang.percentage.toFixed(1)}%</text>
        </g>
      `;
    })
    .join('');

  const emptyMessage =
    data.languages.length === 0
      ? `<text x="${CARD_WIDTH / 2}" y="${CARD_HEIGHT / 2}" text-anchor="middle" class="muted">No language data available</text>`
      : '';

  const content = `
    ${titleSection}
    ${donutSegments}
    ${legendItems}
    ${emptyMessage}
  `;

  return createCardWrapper(CARD_WIDTH, CARD_HEIGHT, content, theme, options);
}
```

**Step 4: Run test to verify it passes**

Run: `pnpm vitest run src/cards/langs-card.test.ts`
Expected: All 5 tests PASS

**Step 5: Commit**

```bash
git add src/cards/langs-card.ts src/cards/langs-card.test.ts
git commit -m "feat: add top languages card with donut chart and legend"
```

---

## Task 13: Route Handlers

**Files:**

- Create: `src/routes/health.ts`
- Create: `src/routes/stats.ts`
- Create: `src/routes/streak.ts`
- Create: `src/routes/top-langs.ts`

**Step 1: Write health route**

```typescript
// src/routes/health.ts
import { Hono } from 'hono';

export const healthRoute = new Hono();

healthRoute.get('/health', (c) => {
  return c.json({ status: 'ok', timestamp: new Date().toISOString() });
});
```

**Step 2: Write card response helper**

Each card route follows the same pattern. Write a shared helper first:

```typescript
// src/routes/card-response.ts
import type { Context } from 'hono';
import { createHash } from 'node:crypto';

export function svgResponse(c: Context, svg: string, cacheSeconds: number): Response {
  const etag = createHash('md5').update(svg).digest('hex');

  c.header('Content-Type', 'image/svg+xml; charset=utf-8');
  c.header(
    'Cache-Control',
    `public, max-age=${cacheSeconds}, s-maxage=${cacheSeconds}, stale-while-revalidate=86400`,
  );
  c.header('ETag', `"${etag}"`);

  return c.body(svg);
}

export function errorSvg(message: string, width = 495, height = 120): string {
  return `
<svg width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" xmlns="http://www.w3.org/2000/svg">
  <rect width="${width}" height="${height}" rx="8" fill="#fef2f2" stroke="#fca5a5" />
  <text x="${width / 2}" y="${height / 2}" text-anchor="middle" dominant-baseline="central"
        font-family="'Segoe UI', system-ui, sans-serif" font-size="14" fill="#dc2626">
    ${message}
  </text>
</svg>`.trim();
}
```

**Step 3: Write stats route**

```typescript
// src/routes/stats.ts
import { Hono } from 'hono';
import type { Cache } from '../cache/index.js';
import { renderStatsCard } from '../cards/stats-card.js';
import { fetchGitHubData, GitHubApiError } from '../fetchers/github.js';
import type { AppConfig } from '../config.js';
import { createPatPool } from '../utils/pat-pool.js';
import { parseCardOptions } from '../utils/query-params.js';
import { createHash } from 'node:crypto';
import { errorSvg, svgResponse } from './card-response.js';

export function createStatsRoute(config: AppConfig, cache: Cache): Hono {
  const app = new Hono();
  const patPool = createPatPool(config.pats);

  app.get('/:username', async (c) => {
    const username = c.req.param('username');
    const options = parseCardOptions(c.req.query());

    const paramsHash = createHash('md5').update(JSON.stringify(options)).digest('hex').slice(0, 8);
    const cacheKey = `card:stats:${username}:${paramsHash}`;

    const cached = await cache.get(cacheKey);
    if (cached) {
      return svgResponse(c, cached, options.cacheSeconds);
    }

    try {
      const token = patPool.getNextToken();
      const data = await fetchGitHubData(username, token);
      const svg = renderStatsCard(data.stats, options);
      await cache.set(cacheKey, svg, options.cacheSeconds);
      return svgResponse(c, svg, options.cacheSeconds);
    } catch (err) {
      if (err instanceof GitHubApiError) {
        if (err.status === 404) {
          const svg = errorSvg(`User not found: ${username}`);
          return svgResponse(c, svg, 300);
        }
        if (err.status === 403) {
          // Try with next token
          try {
            patPool.markExhausted(patPool.getNextToken());
            const retryToken = patPool.getNextToken();
            const data = await fetchGitHubData(username, retryToken);
            const svg = renderStatsCard(data.stats, options);
            await cache.set(cacheKey, svg, options.cacheSeconds);
            return svgResponse(c, svg, options.cacheSeconds);
          } catch {
            c.header('Retry-After', '3600');
            return c.body(errorSvg('Rate limited — try again later'), 429);
          }
        }
      }
      const svg = errorSvg('Something went wrong');
      return c.body(svg, 502, { 'Content-Type': 'image/svg+xml; charset=utf-8' });
    }
  });

  return app;
}
```

**Step 4: Write streak route**

```typescript
// src/routes/streak.ts
import { Hono } from 'hono';
import type { Cache } from '../cache/index.js';
import { renderStreakCard } from '../cards/streak-card.js';
import { fetchGitHubData, GitHubApiError } from '../fetchers/github.js';
import type { AppConfig } from '../config.js';
import { createPatPool } from '../utils/pat-pool.js';
import { parseCardOptions } from '../utils/query-params.js';
import { createHash } from 'node:crypto';
import { errorSvg, svgResponse } from './card-response.js';

export function createStreakRoute(config: AppConfig, cache: Cache): Hono {
  const app = new Hono();
  const patPool = createPatPool(config.pats);

  app.get('/:username/streak', async (c) => {
    const username = c.req.param('username');
    const options = parseCardOptions(c.req.query());

    const paramsHash = createHash('md5').update(JSON.stringify(options)).digest('hex').slice(0, 8);
    const cacheKey = `card:streak:${username}:${paramsHash}`;

    const cached = await cache.get(cacheKey);
    if (cached) {
      return svgResponse(c, cached, options.cacheSeconds);
    }

    try {
      const token = patPool.getNextToken();
      const data = await fetchGitHubData(username, token);
      const svg = renderStreakCard(data.streak, options);
      await cache.set(cacheKey, svg, options.cacheSeconds);
      return svgResponse(c, svg, options.cacheSeconds);
    } catch (err) {
      if (err instanceof GitHubApiError && err.status === 404) {
        return svgResponse(c, errorSvg(`User not found: ${username}`), 300);
      }
      return c.body(errorSvg('Something went wrong'), 502, {
        'Content-Type': 'image/svg+xml; charset=utf-8',
      });
    }
  });

  return app;
}
```

**Step 5: Write top-langs route**

```typescript
// src/routes/top-langs.ts
import { Hono } from 'hono';
import type { Cache } from '../cache/index.js';
import { renderLangsCard } from '../cards/langs-card.js';
import { fetchGitHubData, GitHubApiError } from '../fetchers/github.js';
import type { AppConfig } from '../config.js';
import { createPatPool } from '../utils/pat-pool.js';
import { parseCardOptions } from '../utils/query-params.js';
import { createHash } from 'node:crypto';
import { errorSvg, svgResponse } from './card-response.js';

export function createTopLangsRoute(config: AppConfig, cache: Cache): Hono {
  const app = new Hono();
  const patPool = createPatPool(config.pats);

  app.get('/:username/top-langs', async (c) => {
    const username = c.req.param('username');
    const options = parseCardOptions(c.req.query());

    const paramsHash = createHash('md5').update(JSON.stringify(options)).digest('hex').slice(0, 8);
    const cacheKey = `card:langs:${username}:${paramsHash}`;

    const cached = await cache.get(cacheKey);
    if (cached) {
      return svgResponse(c, cached, options.cacheSeconds);
    }

    try {
      const token = patPool.getNextToken();
      const data = await fetchGitHubData(username, token);
      const svg = renderLangsCard(data.languages, options);
      await cache.set(cacheKey, svg, options.cacheSeconds);
      return svgResponse(c, svg, options.cacheSeconds);
    } catch (err) {
      if (err instanceof GitHubApiError && err.status === 404) {
        return svgResponse(c, errorSvg(`User not found: ${username}`), 300);
      }
      return c.body(errorSvg('Something went wrong'), 502, {
        'Content-Type': 'image/svg+xml; charset=utf-8',
      });
    }
  });

  return app;
}
```

**Step 6: Commit**

```bash
git add src/routes/
git commit -m "feat: add route handlers for health, stats, streak, and top-langs"
```

---

## Task 14: App Entry Point

**Files:**

- Create: `src/index.ts`

**Step 1: Write the Hono app entry**

```typescript
// src/index.ts
import { serve } from '@hono/node-server';
import { Hono } from 'hono';
import { loadConfig } from './config.js';
import { createCache } from './cache/index.js';
import { healthRoute } from './routes/health.js';
import { createStatsRoute } from './routes/stats.js';
import { createStreakRoute } from './routes/streak.js';
import { createTopLangsRoute } from './routes/top-langs.js';

const config = loadConfig();
const cache = createCache(config.redisUrl);

const app = new Hono();

// Health check
app.route('', healthRoute);

// Card routes
const statsRoute = createStatsRoute(config, cache);
const streakRoute = createStreakRoute(config, cache);
const topLangsRoute = createTopLangsRoute(config, cache);

app.route('', statsRoute);
app.route('', streakRoute);
app.route('', topLangsRoute);

// 404 fallback
app.notFound((c) => {
  return c.json({ error: 'Not found' }, 404);
});

// Error handler
app.onError((err, c) => {
  console.error('Unhandled error:', err);
  return c.json({ error: 'Internal server error' }, 500);
});

const port = config.port;
console.log(`Server starting on port ${port}`);

serve({
  fetch: app.fetch,
  port,
});

export default app;
```

> **Context7:** Verify `@hono/node-server` `serve()` options. Confirm it accepts `{ fetch, port }`. Also verify how Hono's `app.route()` merging works with sub-apps.

**Step 2: Test that the app starts**

Run: `pnpm build`
Expected: Build succeeds, creates `dist/index.js`

Run: `PAT_1=test_token node dist/index.js` (then Ctrl+C after verifying startup message)
Expected: "Server starting on port 3000"

**Step 3: Commit**

```bash
git add src/index.ts
git commit -m "feat: add Hono app entry point with route registration"
```

---

## Task 15: Docker Setup

**Files:**

- Create: `Dockerfile`
- Create: `docker-compose.yml`
- Create: `.dockerignore`

**Step 1: Create Dockerfile**

```dockerfile
# Build stage
FROM node:24-alpine AS builder

RUN corepack enable && corepack prepare pnpm@10.5.2 --activate

WORKDIR /app

COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile

COPY tsconfig.json vite.config.ts ./
COPY src/ src/

RUN pnpm build

# Production stage
FROM node:24-alpine AS runner

RUN corepack enable && corepack prepare pnpm@10.5.2 --activate

RUN addgroup -g 1001 -S app && adduser -S app -u 1001

WORKDIR /app

COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile --prod

COPY --from=builder /app/dist ./dist

USER app

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://localhost:3000/health || exit 1

CMD ["node", "dist/index.js"]
```

**Step 2: Create docker-compose.yml**

```yaml
services:
  app:
    build: .
    ports:
      - '3000:3000'
    environment:
      - PAT_1=${PAT_1}
      - PORT=3000
      - REDIS_URL=redis://redis:6379
      - CACHE_TTL=14400
      - LOG_LEVEL=info
    depends_on:
      redis:
        condition: service_healthy
    restart: unless-stopped
    healthcheck:
      test: ['CMD', 'wget', '--no-verbose', '--tries=1', '--spider', 'http://localhost:3000/health']
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s

  redis:
    image: redis:7-alpine
    volumes:
      - redis-data:/data
    restart: unless-stopped
    healthcheck:
      test: ['CMD', 'redis-cli', 'ping']
      interval: 10s
      timeout: 5s
      retries: 3

volumes:
  redis-data:
```

**Step 3: Create .dockerignore**

```
node_modules
dist
.git
.env
*.md
.DS_Store
.vscode
```

**Step 4: Commit**

```bash
git add Dockerfile docker-compose.yml .dockerignore
git commit -m "feat: add Docker multi-stage build and docker compose with Redis"
```

---

## Task 16: Integration Smoke Test

**Files:**

- Create: `src/routes/health.test.ts`

**Step 1: Write integration test for health endpoint**

```typescript
import { describe, expect, it } from 'vitest';
import { Hono } from 'hono';
import { healthRoute } from './health.js';

describe('GET /health', () => {
  it('returns 200 with status ok', async () => {
    const app = new Hono();
    app.route('', healthRoute);

    const res = await app.request('/health');

    expect(res.status).toBe(200);
    const json = await res.json();
    expect(json.status).toBe('ok');
    expect(json.timestamp).toBeDefined();
  });
});
```

**Step 2: Run all tests**

Run: `pnpm vitest run`
Expected: All tests PASS across all test files

**Step 3: Run full check**

Run: `pnpm check`
Expected: Format check passes, lint passes, typecheck passes

**Step 4: Commit**

```bash
git add src/routes/health.test.ts
git commit -m "test: add health endpoint integration test"
```

---

## Task 17: Final Verification & Cleanup

**Step 1: Run the full test suite**

Run: `pnpm test`
Expected: All tests pass

**Step 2: Run the full check pipeline**

Run: `pnpm check`
Expected: All checks pass (format, lint, typecheck)

**Step 3: Verify the dev server starts**

Run: `PAT_1=ghp_test pnpm dev`
Expected: Vite dev server starts, HMR ready

Navigate to `http://localhost:5173/health` (or whatever port Vite uses)
Expected: JSON response `{ "status": "ok", ... }`

**Step 4: Verify the production build**

Run: `pnpm build && PAT_1=ghp_test pnpm start`
Expected: Server starts on port 3000

**Step 5: Update .gitignore if needed**

Ensure `.gitignore` includes:

```
node_modules/
dist/
.env
.DS_Store
```

**Step 6: Final commit**

```bash
git add -A
git commit -m "chore: final cleanup and verification"
```

---

## Summary

| Task | What           | Files          | Tests    |
| ---- | -------------- | -------------- | -------- |
| 1    | Project init   | 6 config files | —        |
| 2    | HTML sanitize  | 2              | 7 tests  |
| 3    | PAT pool       | 2              | 6 tests  |
| 4    | Config & types | 3              | 3 tests  |
| 5    | Query params   | 2              | 7 tests  |
| 6    | Themes         | 3              | 5 tests  |
| 7    | Cache layer    | 4              | 3 tests  |
| 8    | GitHub fetcher | 3              | 4 tests  |
| 9    | Base card      | 2              | 5+ tests |
| 10   | Stats card     | 2              | 6 tests  |
| 11   | Streak card    | 2              | 4 tests  |
| 12   | Langs card     | 2              | 5 tests  |
| 13   | Routes         | 5              | —        |
| 14   | App entry      | 1              | —        |
| 15   | Docker         | 3              | —        |
| 16   | Smoke test     | 1              | 1 test   |
| 17   | Verification   | —              | —        |

**Total: ~42 files, ~56 tests, 17 tasks, ~17 commits**
