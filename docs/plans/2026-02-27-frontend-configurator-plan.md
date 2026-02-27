# Frontend Configurator + Two-Layer Cache Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a two-layer cache (data + SVG) and a React SPA configurator with live preview and drag-and-drop card ordering.

**Architecture:** The backend gains a data cache layer (GitHub API JSON, 4h TTL) sitting in front of the existing SVG cache. A new `/api/data/:username` endpoint exposes cached JSON. The React SPA lives in `frontend/`, is built with Vite, and served by Hono as static files. The configurator uses `<img>` tags pointing at the existing SVG endpoints for pixel-accurate live preview, with `@dnd-kit` for sortable card reordering.

**Tech Stack:** React 19, Vite 8, @dnd-kit/core + @dnd-kit/sortable, @hono/node-server/serve-static

**Key docs:**

- dnd-kit: `DndContext` + `SortableContext` + `useSortable` + `arrayMove` + `verticalListSortingStrategy`
- Hono Node.js static: `import { serveStatic } from '@hono/node-server/serve-static'`
- Themes: `src/themes/index.ts` exports `THEME_NAMES` and `getTheme(name)`
- Query params: `src/utils/query-params.ts` — Zod schema defines all card options

---

### Task 1: Install frontend dependencies

**Files:**

- Modify: `package.json`

**Step 1: Install React + dnd-kit + types**

Run:

```bash
pnpm add react react-dom @dnd-kit/core @dnd-kit/sortable @dnd-kit/utilities
pnpm add -D @types/react @types/react-dom
```

**Step 2: Commit**

```bash
git add package.json pnpm-lock.yaml
git commit -m "chore: add react, dnd-kit, and type dependencies"
```

---

### Task 2: Two-layer cache — refactor card-factory to use data cache

**Files:**

- Modify: `src/routes/card-factory.ts:27-70`
- Test: `src/routes/card-factory.test.ts` (create)

**Step 1: Write the failing test**

Create `src/routes/card-factory.test.ts`:

```typescript
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Hono } from 'hono';

// Mock fetch globally before importing modules that use it
vi.stubGlobal(
  'fetch',
  vi.fn().mockResolvedValue({
    ok: true,
    json: () =>
      Promise.resolve({
        data: {
          user: {
            name: 'Test',
            login: 'testuser',
            contributionsCollection: {
              totalCommitContributions: 10,
              restrictedContributionsCount: 0,
              contributionCalendar: { totalContributions: 10, weeks: [] },
            },
            repositories: { totalCount: 1, nodes: [] },
            pullRequests: { totalCount: 0 },
            issues: { totalCount: 0 },
            followers: { totalCount: 0 },
            repositoriesContributedTo: { totalCount: 0 },
          },
        },
      }),
    headers: new Headers(),
  }),
);

describe('card-factory two-layer cache', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('caches GitHub data and reuses it on SVG cache miss with different options', async () => {
    const { createCardRoute } = await import('./card-factory');
    const { createCache } = await import('../cache/index');

    const cache = createCache(undefined);
    const renderFn = vi.fn().mockReturnValue('<svg>test</svg>');

    const config = {
      pats: ['fake-token'],
      port: 3000,
      cacheTtl: 60,
      logLevel: 'info' as const,
      redisUrl: undefined,
      sentryDsn: undefined,
      sentryTracesSampleRate: 0,
      sentryEnvironment: 'test',
    };

    const app = new Hono();
    app.route(
      '',
      createCardRoute(config, cache, {
        path: '/stats/:username',
        cachePrefix: 'stats',
        render: renderFn,
      }),
    );

    // First request — cache miss, calls GitHub API
    const res1 = await app.request('/stats/testuser');
    expect(res1.status).toBe(200);
    expect(fetch).toHaveBeenCalledTimes(1);

    // Second request with different theme — SVG cache miss, but data cache hit
    const res2 = await app.request('/stats/testuser?theme=dark');
    expect(res2.status).toBe(200);
    // Should NOT have called fetch again — data was cached
    expect(fetch).toHaveBeenCalledTimes(1);
    // But render should have been called twice (different options)
    expect(renderFn).toHaveBeenCalledTimes(2);
  });

  it('returns cached SVG on full cache hit without rendering', async () => {
    const { createCardRoute } = await import('./card-factory');
    const { createCache } = await import('../cache/index');

    const cache = createCache(undefined);
    const renderFn = vi.fn().mockReturnValue('<svg>test</svg>');

    const config = {
      pats: ['fake-token'],
      port: 3000,
      cacheTtl: 60,
      logLevel: 'info' as const,
      redisUrl: undefined,
      sentryDsn: undefined,
      sentryTracesSampleRate: 0,
      sentryEnvironment: 'test',
    };

    const app = new Hono();
    app.route(
      '',
      createCardRoute(config, cache, {
        path: '/stats/:username',
        cachePrefix: 'stats',
        render: renderFn,
      }),
    );

    // First request
    await app.request('/stats/testuser');
    // Second request with same options — full SVG cache hit
    await app.request('/stats/testuser');

    expect(renderFn).toHaveBeenCalledTimes(1);
    expect(fetch).toHaveBeenCalledTimes(1);
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm test -- src/routes/card-factory.test.ts`
Expected: FAIL — second request with `?theme=dark` still calls `fetch` because data cache doesn't exist yet.

**Step 3: Implement two-layer cache in card-factory**

Replace the request flow in `src/routes/card-factory.ts` (lines 27-95). The key changes:

1. Check SVG cache first (existing behavior, just rename key prefix to `svg:`)
2. On SVG miss, check data cache (`data:{username}`)
3. On data miss, fetch from GitHub and cache the data
4. Render SVG from data, cache SVG, return

```typescript
// In createCardRoute, replace the route handler body:
app.get(routeConfig.path, async (c) => {
  const username = c.req.param('username') as string;
  const options = parseCardOptions(c.req.query());

  const paramsHash = createHash('md5').update(JSON.stringify(options)).digest('hex').slice(0, 8);
  const svgCacheKey = `svg:${routeConfig.cachePrefix}:${username}:${paramsHash}`;

  // Layer 2: SVG cache
  const cachedSvg = await cache.get(svgCacheKey);
  if (cachedSvg) {
    return svgResponse(c, cachedSvg, options.cacheSeconds);
  }

  // Layer 1: Data cache
  return Sentry.startSpan(
    {
      name: `card.generate ${routeConfig.cachePrefix}`,
      op: 'card.generate',
      attributes: { username, card_type: routeConfig.cachePrefix },
    },
    async () => {
      try {
        const dataCacheKey = `data:${username}`;
        let data: FetchResult;

        const cachedData = await cache.get(dataCacheKey);
        if (cachedData) {
          data = JSON.parse(cachedData);
        } else {
          const token = patPool.getNextToken();
          try {
            data = await fetchGitHubData(username, token);
          } catch (err) {
            if (err instanceof GitHubRateLimitError) {
              patPool.markExhausted(token);
              const retryToken = patPool.getNextToken();
              data = await fetchGitHubData(username, retryToken);
            } else {
              throw err;
            }
          }
          await cache.set(dataCacheKey, JSON.stringify(data), config.cacheTtl);
        }

        const svg = routeConfig.render(data, options);
        await cache.set(svgCacheKey, svg, options.cacheSeconds);
        Sentry.metrics.count('cards_generated_total', 1, {
          attributes: { card_type: routeConfig.cachePrefix },
        });
        return svgResponse(c, svg, options.cacheSeconds);
      } catch (err) {
        // ... keep existing error handling unchanged ...
      }
    },
  );
});
```

**Step 4: Run tests to verify they pass**

Run: `pnpm test -- src/routes/card-factory.test.ts`
Expected: PASS

**Step 5: Run full check**

Run: `pnpm check && pnpm build && pnpm test`
Expected: All pass

**Step 6: Commit**

```bash
git add src/routes/card-factory.ts src/routes/card-factory.test.ts
git commit -m "feat: add two-layer cache (data + SVG) to card factory"
```

---

### Task 3: Data API endpoint

**Files:**

- Create: `src/routes/data.ts`
- Create: `src/routes/data.test.ts`
- Modify: `src/index.ts:8-26`

**Step 1: Write the failing test**

Create `src/routes/data.test.ts`:

```typescript
import { afterEach, describe, expect, it, vi } from 'vitest';
import { Hono } from 'hono';

vi.stubGlobal(
  'fetch',
  vi.fn().mockResolvedValue({
    ok: true,
    json: () =>
      Promise.resolve({
        data: {
          user: {
            name: 'Test User',
            login: 'testuser',
            contributionsCollection: {
              totalCommitContributions: 100,
              restrictedContributionsCount: 10,
              contributionCalendar: { totalContributions: 500, weeks: [] },
            },
            repositories: { totalCount: 20, nodes: [] },
            pullRequests: { totalCount: 30 },
            issues: { totalCount: 15 },
            followers: { totalCount: 200 },
            repositoriesContributedTo: { totalCount: 5 },
          },
        },
      }),
    headers: new Headers(),
  }),
);

describe('GET /api/data/:username', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('returns JSON with stats, streak, and languages', async () => {
    const { createDataRoute } = await import('./data');
    const { createCache } = await import('../cache/index');

    const config = {
      pats: ['fake-token'],
      port: 3000,
      cacheTtl: 60,
      logLevel: 'info' as const,
      redisUrl: undefined,
      sentryDsn: undefined,
      sentryTracesSampleRate: 0,
      sentryEnvironment: 'test',
    };
    const cache = createCache(undefined);

    const app = new Hono();
    app.route('', createDataRoute(config, cache));

    const res = await app.request('/api/data/testuser');
    expect(res.status).toBe(200);
    expect(res.headers.get('content-type')).toContain('application/json');

    const json = await res.json();
    expect(json.stats.username).toBe('testuser');
    expect(json.streak.username).toBe('testuser');
    expect(json.languages.username).toBe('testuser');
  });

  it('returns 404 for non-existent user', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: () =>
          Promise.resolve({
            data: { user: null },
            errors: [{ message: "Could not resolve to a User with the login of 'ghost'" }],
          }),
        headers: new Headers(),
      }),
    );

    const { createDataRoute } = await import('./data');
    const { createCache } = await import('../cache/index');

    const config = {
      pats: ['fake-token'],
      port: 3000,
      cacheTtl: 60,
      logLevel: 'info' as const,
      redisUrl: undefined,
      sentryDsn: undefined,
      sentryTracesSampleRate: 0,
      sentryEnvironment: 'test',
    };
    const cache = createCache(undefined);

    const app = new Hono();
    app.route('', createDataRoute(config, cache));

    const res = await app.request('/api/data/ghost');
    expect(res.status).toBe(404);
  });
});
```

**Step 2: Run test to verify it fails**

Run: `pnpm test -- src/routes/data.test.ts`
Expected: FAIL — module not found

**Step 3: Implement the data route**

Create `src/routes/data.ts`:

```typescript
import { Hono } from 'hono';
import type { AppConfig } from '../config';
import type { Cache } from '../cache/index';
import { createPatPool } from '../utils/pat-pool';
import { fetchGitHubData, GitHubNotFoundError, GitHubRateLimitError } from '../fetchers/github';
import type { FetchResult } from '../fetchers/github';

export function createDataRoute(config: AppConfig, cache: Cache): Hono {
  const app = new Hono();
  const patPool = createPatPool(config.pats);

  app.get('/api/data/:username', async (c) => {
    const username = c.req.param('username') as string;
    const dataCacheKey = `data:${username}`;

    try {
      const cached = await cache.get(dataCacheKey);
      if (cached) {
        return c.json(JSON.parse(cached));
      }

      const token = patPool.getNextToken();
      let data: FetchResult;

      try {
        data = await fetchGitHubData(username, token);
      } catch (err) {
        if (err instanceof GitHubRateLimitError) {
          patPool.markExhausted(token);
          const retryToken = patPool.getNextToken();
          data = await fetchGitHubData(username, retryToken);
        } else {
          throw err;
        }
      }

      await cache.set(dataCacheKey, JSON.stringify(data), config.cacheTtl);
      return c.json(data);
    } catch (err) {
      if (err instanceof GitHubNotFoundError) {
        return c.json({ error: `User not found: ${username}` }, 404);
      }
      if (err instanceof GitHubRateLimitError) {
        return c.json({ error: 'GitHub API rate limit exceeded' }, 429);
      }
      return c.json({ error: 'Internal server error' }, 500);
    }
  });

  return app;
}
```

**Step 4: Register in index.ts**

Add to `src/index.ts` after the card route imports:

```typescript
import { createDataRoute } from './routes/data';
```

And register it before the card routes:

```typescript
app.route('', createDataRoute(config, cache));
```

**Step 5: Run tests**

Run: `pnpm test -- src/routes/data.test.ts`
Expected: PASS

**Step 6: Run full check**

Run: `pnpm check && pnpm build && pnpm test`

**Step 7: Commit**

```bash
git add src/routes/data.ts src/routes/data.test.ts src/index.ts
git commit -m "feat: add /api/data/:username endpoint for cached GitHub data"
```

---

### Task 4: Themes list API endpoint

The frontend needs to know which themes are available. Expose the theme names via a lightweight JSON endpoint.

**Files:**

- Modify: `src/routes/data.ts`
- Modify: `src/routes/data.test.ts`

**Step 1: Add test**

Add to `src/routes/data.test.ts`:

```typescript
describe('GET /api/themes', () => {
  it('returns array of theme names', async () => {
    const { createDataRoute } = await import('./data');
    const { createCache } = await import('../cache/index');

    const config = {
      pats: ['fake-token'],
      port: 3000,
      cacheTtl: 60,
      logLevel: 'info' as const,
      redisUrl: undefined,
      sentryDsn: undefined,
      sentryTracesSampleRate: 0,
      sentryEnvironment: 'test',
    };
    const cache = createCache(undefined);

    const app = new Hono();
    app.route('', createDataRoute(config, cache));

    const res = await app.request('/api/themes');
    expect(res.status).toBe(200);
    const json = await res.json();
    expect(Array.isArray(json)).toBe(true);
    expect(json).toContain('default');
    expect(json).toContain('dark');
    expect(json).toContain('dracula');
  });
});
```

**Step 2: Run test — expect FAIL**

**Step 3: Add the endpoint to `src/routes/data.ts`**

Import `THEME_NAMES` from `../themes/index` and add:

```typescript
app.get('/api/themes', (c) => {
  return c.json(THEME_NAMES);
});
```

**Step 4: Run tests — expect PASS**

**Step 5: Commit**

```bash
git add src/routes/data.ts src/routes/data.test.ts
git commit -m "feat: add /api/themes endpoint"
```

---

### Task 5: Vite config for dual server + client build

**Files:**

- Modify: `vite.config.ts`
- Create: `frontend/index.html`
- Create: `frontend/src/main.tsx`
- Create: `frontend/tsconfig.json`
- Modify: `tsconfig.json` (exclude frontend/)
- Modify: `package.json` (add `build:client` script)
- Modify: `Dockerfile` (copy frontend/, build client)

**Step 1: Create frontend entry files**

Create `frontend/index.html`:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>GitCard — GitHub Stats Card Configurator</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

Create `frontend/src/main.tsx`:

```tsx
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

function App() {
  return <h1>GitCard Configurator</h1>;
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
```

Create `frontend/tsconfig.json`:

```json
{
  "compilerOptions": {
    "strict": true,
    "target": "ES2024",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "jsx": "react-jsx",
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "outDir": "../dist/static",
    "rootDir": "src",
    "types": ["react", "react-dom"]
  },
  "include": ["src/**/*.ts", "src/**/*.tsx"]
}
```

**Step 2: Update root tsconfig.json to exclude frontend/**

Add `"frontend"` to the `exclude` array in `tsconfig.json`.

**Step 3: Add client build script to package.json**

Add to `scripts`:

```json
"build:client": "vite build --config vite.client.config.ts",
"build": "vite build && vite build --config vite.client.config.ts"
```

**Step 4: Create `vite.client.config.ts`**

```typescript
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  root: 'frontend',
  plugins: [react()],
  build: {
    outDir: '../dist/static',
    emptyOutDir: true,
  },
});
```

**Step 5: Install @vitejs/plugin-react**

Run: `pnpm add -D @vitejs/plugin-react`

**Step 6: Serve static files in Hono**

Add to `src/index.ts`, before the 404 handler:

```typescript
import { serveStatic } from '@hono/node-server/serve-static';

// Serve frontend static files
app.use('/*', serveStatic({ root: './dist/static' }));
```

**Step 7: Update Dockerfile**

Add `COPY frontend/ frontend/` and `COPY vite.client.config.ts ./` in the builder stage. The `pnpm build` command now builds both server and client.

**Step 8: Build and verify**

Run: `pnpm build`
Expected: Server builds to `dist/index.js`, client builds to `dist/static/`

**Step 9: Commit**

```bash
git add frontend/ vite.client.config.ts vite.config.ts tsconfig.json package.json Dockerfile src/index.ts
git commit -m "feat: add Vite client build for React SPA frontend"
```

---

### Task 6: useCardConfig hook — state management

**Files:**

- Create: `frontend/src/hooks/useCardConfig.ts`

**Step 1: Create the hook**

This hook manages all configurator state: username, enabled cards, card order, theme, colors, and toggles.

```typescript
import { useState, useCallback, useMemo } from 'react';

export type CardType = 'stats' | 'streak' | 'top-langs';

interface CardConfig {
  username: string;
  cards: CardType[];
  theme: string;
  showIcons: boolean;
  hideBorder: boolean;
  hideTitle: boolean;
  bgColor: string;
  titleColor: string;
  textColor: string;
  iconColor: string;
  borderColor: string;
  locale: string;
  hide: string[];
}

const DEFAULT_CONFIG: CardConfig = {
  username: '',
  cards: ['stats', 'streak', 'top-langs'],
  theme: 'default',
  showIcons: true,
  hideBorder: false,
  hideTitle: false,
  bgColor: '',
  titleColor: '',
  textColor: '',
  iconColor: '',
  borderColor: '',
  locale: 'en',
  hide: [],
};

export function useCardConfig() {
  const [config, setConfig] = useState<CardConfig>(DEFAULT_CONFIG);

  const setUsername = useCallback((username: string) => {
    setConfig((prev) => ({ ...prev, username }));
  }, []);

  const setCards = useCallback((cards: CardType[]) => {
    setConfig((prev) => ({ ...prev, cards }));
  }, []);

  const toggleCard = useCallback((card: CardType) => {
    setConfig((prev) => ({
      ...prev,
      cards: prev.cards.includes(card)
        ? prev.cards.filter((c) => c !== card)
        : [...prev.cards, card],
    }));
  }, []);

  const setTheme = useCallback((theme: string) => {
    setConfig((prev) => ({ ...prev, theme }));
  }, []);

  const setOption = useCallback(<K extends keyof CardConfig>(key: K, value: CardConfig[K]) => {
    setConfig((prev) => ({ ...prev, [key]: value }));
  }, []);

  const buildQueryString = useCallback(
    (cardType: CardType): string => {
      const params = new URLSearchParams();
      if (config.theme !== 'default') params.set('theme', config.theme);
      if (!config.showIcons) params.set('show_icons', 'false');
      if (config.hideBorder) params.set('hide_border', 'true');
      if (config.hideTitle) params.set('hide_title', 'true');
      if (config.bgColor) params.set('bg_color', config.bgColor);
      if (config.titleColor) params.set('title_color', config.titleColor);
      if (config.textColor) params.set('text_color', config.textColor);
      if (config.iconColor) params.set('icon_color', config.iconColor);
      if (config.borderColor) params.set('border_color', config.borderColor);
      if (config.locale !== 'en') params.set('locale', config.locale);
      if (cardType === 'stats' && config.hide.length > 0) {
        params.set('hide', config.hide.join(','));
      }
      const qs = params.toString();
      return qs ? `?${qs}` : '';
    },
    [config],
  );

  const cardPaths: Record<CardType, string> = useMemo(
    () => ({
      stats: `/stats/${config.username}`,
      streak: `/stats/${config.username}/streak`,
      'top-langs': `/stats/${config.username}/top-langs`,
    }),
    [config.username],
  );

  return {
    config,
    setUsername,
    setCards,
    toggleCard,
    setTheme,
    setOption,
    buildQueryString,
    cardPaths,
  };
}
```

**Step 2: Commit**

```bash
git add frontend/src/hooks/useCardConfig.ts
git commit -m "feat: add useCardConfig hook for configurator state"
```

---

### Task 7: Sidebar component — controls panel

**Files:**

- Create: `frontend/src/components/Sidebar.tsx`
- Create: `frontend/src/components/ThemePicker.tsx`
- Create: `frontend/src/components/ColorPicker.tsx`
- Create: `frontend/src/components/OptionToggles.tsx`

**Step 1: Create ThemePicker**

```tsx
import { useEffect, useState } from 'react';

interface ThemePickerProps {
  value: string;
  onChange: (theme: string) => void;
}

export function ThemePicker({ value, onChange }: ThemePickerProps) {
  const [themes, setThemes] = useState<string[]>(['default', 'dark', 'dracula']);

  useEffect(() => {
    fetch('/api/themes')
      .then((res) => res.json())
      .then((data) => setThemes(data))
      .catch(() => {});
  }, []);

  return (
    <label className="control-group">
      <span className="control-label">Theme</span>
      <select value={value} onChange={(e) => onChange(e.target.value)}>
        {themes.map((t) => (
          <option key={t} value={t}>
            {t}
          </option>
        ))}
      </select>
    </label>
  );
}
```

**Step 2: Create ColorPicker**

```tsx
interface ColorPickerProps {
  label: string;
  value: string;
  onChange: (color: string) => void;
}

export function ColorPicker({ label, value, onChange }: ColorPickerProps) {
  return (
    <label className="control-group">
      <span className="control-label">{label}</span>
      <div className="color-input-row">
        <input
          type="color"
          value={value ? `#${value}` : '#000000'}
          onChange={(e) => onChange(e.target.value.replace('#', ''))}
        />
        <input
          type="text"
          value={value}
          onChange={(e) => onChange(e.target.value.replace('#', ''))}
          placeholder="hex without #"
          maxLength={6}
        />
      </div>
    </label>
  );
}
```

**Step 3: Create OptionToggles**

```tsx
interface OptionTogglesProps {
  showIcons: boolean;
  hideBorder: boolean;
  hideTitle: boolean;
  onToggle: (key: 'showIcons' | 'hideBorder' | 'hideTitle', value: boolean) => void;
}

export function OptionToggles({ showIcons, hideBorder, hideTitle, onToggle }: OptionTogglesProps) {
  return (
    <fieldset className="control-group">
      <legend className="control-label">Options</legend>
      <label>
        <input
          type="checkbox"
          checked={showIcons}
          onChange={(e) => onToggle('showIcons', e.target.checked)}
        />
        Show icons
      </label>
      <label>
        <input
          type="checkbox"
          checked={hideBorder}
          onChange={(e) => onToggle('hideBorder', e.target.checked)}
        />
        Hide border
      </label>
      <label>
        <input
          type="checkbox"
          checked={hideTitle}
          onChange={(e) => onToggle('hideTitle', e.target.checked)}
        />
        Hide title
      </label>
    </fieldset>
  );
}
```

**Step 4: Create Sidebar** (composes the above)

```tsx
import type { CardType } from '../hooks/useCardConfig';
import { ThemePicker } from './ThemePicker';
import { ColorPicker } from './ColorPicker';
import { OptionToggles } from './OptionToggles';

interface SidebarProps {
  username: string;
  onUsernameChange: (username: string) => void;
  cards: CardType[];
  onToggleCard: (card: CardType) => void;
  theme: string;
  onThemeChange: (theme: string) => void;
  showIcons: boolean;
  hideBorder: boolean;
  hideTitle: boolean;
  onToggleOption: (key: 'showIcons' | 'hideBorder' | 'hideTitle', value: boolean) => void;
  bgColor: string;
  titleColor: string;
  textColor: string;
  iconColor: string;
  borderColor: string;
  onColorChange: (key: string, value: string) => void;
}

const ALL_CARDS: { id: CardType; label: string }[] = [
  { id: 'stats', label: 'Stats' },
  { id: 'streak', label: 'Streak' },
  { id: 'top-langs', label: 'Top Languages' },
];

export function Sidebar(props: SidebarProps) {
  return (
    <aside className="sidebar">
      <label className="control-group">
        <span className="control-label">GitHub Username</span>
        <input
          type="text"
          value={props.username}
          onChange={(e) => props.onUsernameChange(e.target.value)}
          placeholder="e.g. torvalds"
        />
      </label>

      <fieldset className="control-group">
        <legend className="control-label">Cards</legend>
        {ALL_CARDS.map((card) => (
          <label key={card.id}>
            <input
              type="checkbox"
              checked={props.cards.includes(card.id)}
              onChange={() => props.onToggleCard(card.id)}
            />
            {card.label}
          </label>
        ))}
      </fieldset>

      <ThemePicker value={props.theme} onChange={props.onThemeChange} />

      <OptionToggles
        showIcons={props.showIcons}
        hideBorder={props.hideBorder}
        hideTitle={props.hideTitle}
        onToggle={props.onToggleOption}
      />

      <fieldset className="control-group">
        <legend className="control-label">Custom Colors</legend>
        <ColorPicker
          label="Background"
          value={props.bgColor}
          onChange={(v) => props.onColorChange('bgColor', v)}
        />
        <ColorPicker
          label="Title"
          value={props.titleColor}
          onChange={(v) => props.onColorChange('titleColor', v)}
        />
        <ColorPicker
          label="Text"
          value={props.textColor}
          onChange={(v) => props.onColorChange('textColor', v)}
        />
        <ColorPicker
          label="Icon"
          value={props.iconColor}
          onChange={(v) => props.onColorChange('iconColor', v)}
        />
        <ColorPicker
          label="Border"
          value={props.borderColor}
          onChange={(v) => props.onColorChange('borderColor', v)}
        />
      </fieldset>
    </aside>
  );
}
```

**Step 5: Commit**

```bash
git add frontend/src/components/
git commit -m "feat: add Sidebar, ThemePicker, ColorPicker, OptionToggles components"
```

---

### Task 8: CardPreview + CardList with drag-and-drop

**Files:**

- Create: `frontend/src/components/CardPreview.tsx`
- Create: `frontend/src/components/CardList.tsx`

**Step 1: Create CardPreview**

Each card is rendered as an `<img>` tag pointing at the SVG endpoint. The key is set to the full URL so React re-mounts and re-fetches when params change.

```tsx
import type { CardType } from '../hooks/useCardConfig';

interface CardPreviewProps {
  id: CardType;
  src: string;
}

export function CardPreview({ id, src }: CardPreviewProps) {
  return (
    <div className="card-preview">
      <img src={src} alt={`${id} card`} width={495} />
    </div>
  );
}
```

**Step 2: Create CardList with dnd-kit**

```tsx
import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core';
import {
  arrayMove,
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import type { CardType } from '../hooks/useCardConfig';
import { CardPreview } from './CardPreview';

interface SortableCardProps {
  id: CardType;
  src: string;
}

function SortableCard({ id, src }: SortableCardProps) {
  const { attributes, listeners, setNodeRef, transform, transition } = useSortable({ id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div ref={setNodeRef} style={style} {...attributes} {...listeners} className="sortable-card">
      <CardPreview id={id} src={src} />
    </div>
  );
}

interface CardListProps {
  cards: CardType[];
  onReorder: (cards: CardType[]) => void;
  buildSrc: (card: CardType) => string;
}

export function CardList({ cards, onReorder, buildSrc }: CardListProps) {
  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  );

  function handleDragEnd(event: DragEndEvent) {
    const { active, over } = event;
    if (over && active.id !== over.id) {
      const oldIndex = cards.indexOf(active.id as CardType);
      const newIndex = cards.indexOf(over.id as CardType);
      onReorder(arrayMove(cards, oldIndex, newIndex));
    }
  }

  return (
    <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
      <SortableContext items={cards} strategy={verticalListSortingStrategy}>
        <div className="card-list">
          {cards.map((card) => (
            <SortableCard key={card} id={card} src={buildSrc(card)} />
          ))}
        </div>
      </SortableContext>
    </DndContext>
  );
}
```

**Step 3: Commit**

```bash
git add frontend/src/components/CardPreview.tsx frontend/src/components/CardList.tsx
git commit -m "feat: add CardPreview and drag-and-drop CardList components"
```

---

### Task 9: EmbedOutput component

**Files:**

- Create: `frontend/src/components/EmbedOutput.tsx`

**Step 1: Create the component**

```tsx
import { useCallback, useState } from 'react';
import type { CardType } from '../hooks/useCardConfig';

interface EmbedOutputProps {
  cards: CardType[];
  buildSrc: (card: CardType) => string;
  baseUrl: string;
}

const CARD_LABELS: Record<CardType, string> = {
  stats: 'GitHub Stats',
  streak: 'GitHub Streak',
  'top-langs': 'Top Languages',
};

export function EmbedOutput({ cards, buildSrc, baseUrl }: EmbedOutputProps) {
  const [copied, setCopied] = useState<'md' | 'html' | null>(null);

  const markdown = cards
    .map((card) => `![${CARD_LABELS[card]}](${baseUrl}${buildSrc(card)})`)
    .join('\n');

  const html = cards
    .map((card) => `<img src="${baseUrl}${buildSrc(card)}" alt="${CARD_LABELS[card]}" />`)
    .join('\n');

  const copy = useCallback(
    async (format: 'md' | 'html') => {
      const text = format === 'md' ? markdown : html;
      await navigator.clipboard.writeText(text);
      setCopied(format);
      setTimeout(() => setCopied(null), 2000);
    },
    [markdown, html],
  );

  return (
    <div className="embed-output">
      <div className="embed-actions">
        <button onClick={() => copy('md')} className="copy-btn">
          {copied === 'md' ? 'Copied!' : 'Copy Markdown'}
        </button>
        <button onClick={() => copy('html')} className="copy-btn">
          {copied === 'html' ? 'Copied!' : 'Copy HTML'}
        </button>
      </div>
      <pre className="embed-code">
        <code>{markdown}</code>
      </pre>
    </div>
  );
}
```

**Step 2: Commit**

```bash
git add frontend/src/components/EmbedOutput.tsx
git commit -m "feat: add EmbedOutput component with markdown/HTML copy"
```

---

### Task 10: App component — wire everything together

**Files:**

- Modify: `frontend/src/main.tsx`
- Create: `frontend/src/App.tsx`

**Step 1: Create App.tsx**

```tsx
import { useState, useEffect, useCallback } from 'react';
import { useCardConfig } from './hooks/useCardConfig';
import { Sidebar } from './components/Sidebar';
import { CardList } from './components/CardList';
import { EmbedOutput } from './components/EmbedOutput';
import './styles/app.css';

export function App() {
  const {
    config,
    setUsername,
    setCards,
    toggleCard,
    setTheme,
    setOption,
    buildQueryString,
    cardPaths,
  } = useCardConfig();

  const [debouncedUsername, setDebouncedUsername] = useState('');
  const [isValid, setIsValid] = useState(false);

  // Debounce username input
  useEffect(() => {
    if (!config.username) {
      setDebouncedUsername('');
      setIsValid(false);
      return;
    }
    const timer = setTimeout(() => {
      setDebouncedUsername(config.username);
    }, 300);
    return () => clearTimeout(timer);
  }, [config.username]);

  // Validate username via data API
  useEffect(() => {
    if (!debouncedUsername) {
      setIsValid(false);
      return;
    }
    fetch(`/api/data/${debouncedUsername}`)
      .then((res) => {
        setIsValid(res.ok);
      })
      .catch(() => setIsValid(false));
  }, [debouncedUsername]);

  const buildSrc = useCallback(
    (card: (typeof config.cards)[number]) => {
      return `${cardPaths[card]}${buildQueryString(card)}`;
    },
    [cardPaths, buildQueryString],
  );

  const baseUrl = typeof window !== 'undefined' ? window.location.origin : '';

  return (
    <div className="app">
      <header className="app-header">
        <h1>GitCard</h1>
        <a
          href="https://github.com/Tourniercy/gitcard"
          target="_blank"
          rel="noopener noreferrer"
          className="github-link"
        >
          GitHub
        </a>
      </header>

      <div className="app-body">
        <Sidebar
          username={config.username}
          onUsernameChange={setUsername}
          cards={config.cards}
          onToggleCard={toggleCard}
          theme={config.theme}
          onThemeChange={setTheme}
          showIcons={config.showIcons}
          hideBorder={config.hideBorder}
          hideTitle={config.hideTitle}
          onToggleOption={(key, value) => setOption(key, value)}
          bgColor={config.bgColor}
          titleColor={config.titleColor}
          textColor={config.textColor}
          iconColor={config.iconColor}
          borderColor={config.borderColor}
          onColorChange={(key, value) => setOption(key as keyof typeof config, value)}
        />

        <main className="preview-area">
          {debouncedUsername && isValid ? (
            <CardList cards={config.cards} onReorder={setCards} buildSrc={buildSrc} />
          ) : debouncedUsername && !isValid ? (
            <div className="preview-placeholder">User not found</div>
          ) : (
            <div className="preview-placeholder">Enter a GitHub username to get started</div>
          )}
        </main>
      </div>

      {debouncedUsername && isValid && (
        <EmbedOutput cards={config.cards} buildSrc={buildSrc} baseUrl={baseUrl} />
      )}
    </div>
  );
}
```

**Step 2: Update main.tsx to use App**

```tsx
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './App';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
```

**Step 3: Commit**

```bash
git add frontend/src/App.tsx frontend/src/main.tsx
git commit -m "feat: add App component wiring sidebar, preview, and embed output"
```

---

### Task 11: CSS styling

**Files:**

- Create: `frontend/src/styles/app.css`

**Step 1: Create the stylesheet**

Clean, minimal design. The layout uses CSS grid for the two-column sidebar + preview. Cards have a subtle shadow and drag cursor. Embed output at the bottom with monospace code block.

Key layout rules:

- `.app-body`: `display: grid; grid-template-columns: 280px 1fr; gap: 24px;`
- `.sidebar`: Sticky, scrollable, all controls stacked vertically
- `.card-list`: Flex column, gap between cards
- `.sortable-card`: `cursor: grab;` with `:active { cursor: grabbing; }`
- `.embed-output`: Full-width bottom section with code + copy buttons
- Responsive: Single column below 768px

**Step 2: Commit**

```bash
git add frontend/src/styles/app.css
git commit -m "feat: add CSS styles for configurator layout"
```

---

### Task 12: Update Dockerfile for frontend build

**Files:**

- Modify: `Dockerfile`

**Step 1: Update the builder stage**

The Dockerfile needs to:

1. Copy `frontend/` directory
2. Copy `vite.client.config.ts`
3. The `pnpm build` script already builds both server + client

Add after `COPY src/ src/`:

```dockerfile
COPY frontend/ frontend/
COPY vite.client.config.ts ./
```

**Step 2: Update the runner stage**

The static files are in `dist/static/` which is already copied via `COPY --from=builder /app/dist ./dist`.

**Step 3: Build and test**

Run: `pnpm build` (local) to verify both builds succeed.

**Step 4: Commit**

```bash
git add Dockerfile
git commit -m "chore: update Dockerfile to include frontend build"
```

---

### Task 13: Integration test — full frontend + API flow

**Files:**

- Modify: `src/integration.test.ts`

**Step 1: Add data API integration test**

Add to the existing integration test file:

```typescript
it('returns cached data via /api/data/:username', async () => {
  // ... import and set up app with createDataRoute
  // Verify JSON response with stats, streak, languages
  // Verify second request uses cache (no additional fetch call)
});

it('returns theme list via /api/themes', async () => {
  // Verify array response containing 'default', 'dark', 'dracula'
});
```

**Step 2: Run full check**

Run: `pnpm check && pnpm build && pnpm test`
Expected: All pass, including new integration tests

**Step 3: Commit**

```bash
git add src/integration.test.ts
git commit -m "test: add integration tests for /api/data and /api/themes endpoints"
```

---

### Task 14: Final verification

**Step 1: Run the full check suite**

Run: `pnpm check && pnpm build && pnpm test`

**Step 2: Manual smoke test**

Run: `pnpm dev`

1. Open `http://localhost:3000/` — should see the configurator
2. Enter a username — cards should load
3. Change theme — cards should update (no extra API calls visible in server logs)
4. Drag a card — order should change, embed code should update
5. Click "Copy Markdown" — should copy to clipboard
6. Verify existing SVG endpoints still work: `http://localhost:3000/stats/torvalds`

**Step 3: Final commit if any fixes needed**

```bash
git add -A
git commit -m "fix: address issues found in manual smoke test"
```
