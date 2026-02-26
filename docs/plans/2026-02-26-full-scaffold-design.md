# github-stats-cards — Full Scaffold Design

Date: 2026-02-26

## Overview

Self-hosted API service that generates dynamic SVG stat cards from GitHub user data. Runs on a VPS inside Docker, behind Cloudflare CDN, serving embeddable `<img>` tags in GitHub profile READMEs.

API-only — no frontend, no database, no authentication. Fetches data from GitHub GraphQL API, builds SVG strings, returns them with cache headers.

## Visual Direction

**Data visualization focused** with a **glassmorphism** aesthetic inspired by shadcn/ui's clean design language. Cards use circular charts (rings, donuts) instead of plain text lists, with frosted-glass backgrounds and subtle animations.

Since GitHub's Camo proxy strips CSS `backdrop-filter`, the glass effect is implemented via SVG filters (`feGaussianBlur` + semi-transparent fills + `feComposite`).

## Tech Stack

| Layer | Choice |
|---|---|
| Runtime | Node.js 24 LTS (24.13.x) |
| Framework | Hono + @hono/node-server |
| Language | TypeScript 5.8+ (strict) |
| Build | Vite 8 (beta, Rolldown-powered) |
| Linter | Oxlint 1.x |
| Formatter | Oxfmt (beta) |
| Package manager | pnpm 10+ |
| Version manager | Volta |
| Deployment | Docker + docker compose |
| Memory cache | lru-cache |
| Redis cache | ioredis |
| Validation | Zod |

Excluded: Express, ESLint, Prettier, Biome, axios, node-fetch, React/JSX, any ORM/database, PM2.

## Architecture

```
src/
├── index.ts                 # Hono app entry + server
├── config.ts                # Zod env validation
├── types.ts                 # Shared types
├── routes/
│   ├── stats.ts             # GET /:username
│   ├── streak.ts            # GET /:username/streak
│   ├── top-langs.ts         # GET /:username/top-langs
│   └── health.ts            # GET /health
├── fetchers/
│   ├── github.ts            # GitHub GraphQL client
│   └── types.ts             # API response types
├── cards/
│   ├── base-card.ts         # Shared: glass filter, arc math, card wrapper, formatNumber
│   ├── stats-card.ts        # Stats card SVG builder
│   ├── streak-card.ts       # Streak card SVG builder
│   └── langs-card.ts        # Top languages SVG builder
├── themes/
│   ├── index.ts             # Theme registry + lookup
│   └── themes.ts            # Built-in theme definitions
├── cache/
│   ├── index.ts             # Cache interface + factory
│   ├── memory.ts            # LRU in-memory cache
│   └── redis.ts             # Redis cache layer
└── utils/
    ├── pat-pool.ts          # PAT token round-robin rotation
    ├── sanitize.ts          # HTML/SVG entity escaping
    └── query-params.ts      # Zod query param schema
```

### SVG rendering: hybrid approach

Shared SVG utilities live in `base-card.ts`:
- `createGlassFilter()` — SVG `<defs>` block with glassmorphism filter
- `createArcPath(cx, cy, radius, startAngle, endAngle)` — SVG path `d` for arcs
- `createCardWrapper(width, height, content, options)` — outer `<svg>` with glass bg
- `formatNumber(n, locale)` — locale-aware number formatting

Each card file owns its full layout as a monolithic template literal function.

## API Routes

| Route | Method | Content-Type | Description |
|---|---|---|---|
| `/:username` | GET | `image/svg+xml` | Stats card |
| `/:username/streak` | GET | `image/svg+xml` | Streak card |
| `/:username/top-langs` | GET | `image/svg+xml` | Top languages card |
| `/health` | GET | `application/json` | Docker health check |

### Query parameters

Shared across card routes, validated with Zod:

| Param | Type | Default | Description |
|---|---|---|---|
| `theme` | string | `"default"` | Theme name |
| `hide` | string | `""` | Comma-separated stats to hide |
| `show_icons` | boolean | `true` | Show icons |
| `hide_border` | boolean | `false` | Hide card border |
| `hide_title` | boolean | `false` | Hide card title |
| `bg_color` | string | — | Background color override (hex) |
| `title_color` | string | — | Title color override |
| `text_color` | string | — | Text color override |
| `icon_color` | string | — | Icon color override |
| `border_color` | string | — | Border color override |
| `cache_seconds` | number | `14400` | Cache TTL override (min 1800) |
| `locale` | string | `"en"` | Number format locale |

### Request flow

1. Route handler validates query params (Zod)
2. Check cache: `card:{type}:{username}:{paramsHash}`
3. On miss: fetch from GitHub GraphQL API (PAT from pool)
4. Pass data + options to card builder → SVG string
5. Store in cache, return SVG with headers

### Response headers

```
Content-Type: image/svg+xml; charset=utf-8
Cache-Control: public, max-age=14400, s-maxage=14400, stale-while-revalidate=86400
ETag: <md5 of SVG body>
```

## Card Designs

All cards are 495×195px. Font: `'Segoe UI', system-ui, -apple-system, sans-serif`.

### Stats Card

- **Left:** Circular SVG progress ring showing contribution grade (A+, A, B+, B, C). Grade is calculated from a weighted score of all metrics.
- **Right:** 6 rows — icon + label + right-aligned value:
  - Total Stars, Total Forks, Total Commits, Total PRs, Total Issues, Contributed To
- **Background:** Glassmorphism (SVG filter), subtle border, 8px corner radius
- **Animations:** Subtle fade-in on load (CSS keyframes in `<style>`)

### Top Languages Card

- **Left:** SVG donut chart — multiple arc segments, each colored per language. Hollow center.
- **Right:** Legend — colored dots + language name + percentage. Top 5 languages.
- **Background:** Same glassmorphism treatment.

### Streak Card

- **Layout:** Three columns:
  - Total Contributions (left)
  - Current Streak (center, visually emphasized — larger font, accent color, flame SVG icon)
  - Longest Streak (right)
- **Each column:** Large number, unit label ("days" / "contributions"), date range below in muted text
- **Background:** Same glassmorphism treatment.

### SVG Constraints (GitHub Camo compatibility)

- No `<script>`, `onclick`, `foreignObject`, `<iframe>`
- No external stylesheets or fonts (embedded `<style>` only)
- CSS animations allowed (Camo preserves them)
- No CSS `backdrop-filter` (stripped by Camo) — use SVG filter instead

## Theme System

```typescript
interface Theme {
  name: string;
  background: string;      // card bg (semi-transparent for glass)
  backgroundBlur: string;  // blur backdrop color
  border: string;          // subtle border
  title: string;           // title text color
  text: string;            // body text color
  muted: string;           // secondary text
  icon: string;            // icon/accent color
  ring: string;            // progress ring color
}
```

Initial themes (3):
- `default` — light, clean white glass
- `dark` — dark mode glass
- `dracula` — purple-accented glass

More themes added later.

## Caching

### Interface

```typescript
interface Cache {
  get(key: string): Promise<string | null>;
  set(key: string, value: string, ttlSeconds: number): Promise<void>;
}
```

### Implementations

- **Memory:** `lru-cache`, max 500 entries, default 4h TTL
- **Redis:** `ioredis` via `REDIS_URL`, uses `SETEX`, graceful fallback to memory on connection failure

### Factory

`cache/index.ts` exports a factory: if `REDIS_URL` is set, returns Redis; otherwise returns LRU.

### Cache keys

Format: `card:{type}:{username}:{paramsHash}`

`paramsHash` is MD5 of sorted, relevant query parameters.

## PAT Token Pool

- Reads `PAT_1`, `PAT_2`, ..., `PAT_N` from env at startup
- `getNextToken()` round-robins through available tokens
- On 403 or rate limit (`X-RateLimit-Remaining: 0`): mark token exhausted, try next, retry once
- Each PAT provides 5,000 GraphQL points/hour

## Error Handling

| Condition | Status | Response |
|---|---|---|
| Invalid username (not found) | 404 | Error SVG card |
| GitHub API error | 502 | Error SVG card |
| All PATs rate limited | 429 | Error SVG card + `Retry-After` header |
| Invalid query params | 400 | JSON error (Zod messages) |
| Health check | 200 | JSON status (always succeeds) |

Error SVG cards keep `<img>` tags from visually breaking in READMEs.

## Docker

- **Dockerfile:** Multi-stage (node:24-alpine). Builder stage installs deps + builds. Runner stage copies dist, runs as non-root user. `HEALTHCHECK` calls `/health`.
- **docker-compose.yml:** `app` service (port 3000) + `redis` service (persistent volume). Health checks, `restart: unless-stopped`.
- **.dockerignore:** Excludes `node_modules`, `.git`, `dist`, `.env`.

## Configuration

All env vars loaded in `config.ts` via Zod. No direct `process.env` usage outside this file.

| Var | Required | Default | Description |
|---|---|---|---|
| `PAT_1` | Yes | — | GitHub PAT (at least one) |
| `PAT_2`...`PAT_N` | No | — | Additional PATs |
| `PORT` | No | `3000` | Server port |
| `REDIS_URL` | No | — | Redis connection string |
| `CACHE_TTL` | No | `14400` | Default cache TTL (seconds) |
| `LOG_LEVEL` | No | `"info"` | Log level |
