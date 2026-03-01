# CLAUDE.md

Project context and rules for AI assistants working on this codebase.

## Project overview

**github-stats-cards** is a self-hosted API service that generates dynamic SVG stat cards from GitHub user data. It runs on a VPS inside Docker, behind a CDN (Cloudflare), serving embeddable `<img>` tags in GitHub profile READMEs.

The service is API-only — there is no frontend, no database, no authentication. It fetches data from GitHub's GraphQL API, builds SVG strings, and returns them with cache headers.

## Tech stack

- **Runtime**: Node.js 24 LTS (Krypton, 24.13.x) — V8 13.6, native fetch, global URLPattern
- **Framework**: Hono with `@hono/node-server`
- **Language**: TypeScript (strict mode)
- **Build**: Vite 8 (beta, Rolldown-powered) — configured via `@hono/vite-build/node` and `@hono/vite-dev-server`
- **Linter**: Oxlint 1.x (config: `.oxlintrc.json`)
- **Formatter**: Oxfmt beta (config: `.oxfmtrc.json`)
- **Package manager**: pnpm 10+
- **Version manager**: pnpm (`use-node-version` in `.npmrc` — auto-downloads the pinned Node.js)
- **Deployment**: Docker + docker compose (no PM2, no systemd)

### What we do NOT use

- **No Express, Fastify, or Koa** — Hono only.
- **No ESLint** — Oxlint replaces it entirely. Do not add eslint configs or dependencies.
- **No Prettier or Biome** — Oxfmt is the formatter. Do not add prettier configs or dependencies.
- **No esbuild or Rollup directly** — Vite 8 uses Rolldown internally. Do not add rollup/esbuild configs.
- **No axios or node-fetch** — use native `fetch()` (built into Node.js 24).
- **No React or JSX rendering libraries** — SVG is built via TypeScript template literals.
- **No ORM or database** — caching only (in-memory LRU + Redis via docker compose).
- **No PM2, no systemd** — Docker is the process manager. `restart: unless-stopped` handles crashes.
- **No Volta, nvm, fnm, or n** — pnpm manages Node.js via `use-node-version` in `.npmrc`.

## Documentation lookup — always use Context7

**Before writing or modifying any code, always use Context7 MCP to check the latest documentation for the libraries in this project.** Append `use context7` to your prompt, or call the MCP tools directly:

1. `resolve-library-id` — resolve a library name to a Context7 ID
2. `get-library-docs` / `query-docs` — fetch version-specific docs by topic

This is non-negotiable because several tools in this stack are pre-1.0 or in active beta. Training data is almost certainly outdated for them. Context7 pulls docs straight from source repos.

### When to use Context7

- **Every time** you write or modify code that uses a project dependency
- **Every time** you update a config file (`.oxlintrc.json`, `.oxfmtrc.json`, `vite.config.ts`, `tsconfig.json`)
- **Every time** you're unsure about an API — never guess, always verify
- When adding new dependencies
- When debugging build or lint errors

### Libraries to always check

| Library                 | Context7 lookup topics                                                       |
| ----------------------- | ---------------------------------------------------------------------------- |
| `hono`                  | Routing, middleware, context methods (`c.header()`, `c.body()`, `c.json()`)  |
| `@hono/node-server`     | `serve()` options, port config, graceful shutdown                            |
| `@hono/vite-build`      | Node build plugin, entry/output config                                       |
| `@hono/vite-dev-server` | Dev server setup, `nodeAdapter`, HMR                                         |
| `vite`                  | Vite 8 config — `build.rolldownOptions` (not `rollupOptions`), SSR externals |
| `zod`                   | Schema definitions, `.parse()`, `.safeParse()`, transforms                   |
| `lru-cache`             | Constructor options (`max`, `ttl`), `.get()`, `.set()`                       |
| `ioredis`               | Connection string, `get`, `set`, `setex`, error handling                     |
| `oxlint`                | Config schema, rule names, plugin names, category names                      |
| `oxfmt`                 | Config schema, import sorting options, override patterns                     |

### Example

When creating a new route handler:

```
How does Hono handle path parameters and query strings in route handlers? use context7
```

When updating the Vite config:

```
What are the @hono/vite-build/node options for entry and output in Vite 8? use context7
```

**Do not rely on training data for any library API. Always verify with Context7.**

## Project structure

```
src/
├── index.ts                 # App entry — creates Hono app, registers routes, starts server
├── config.ts                # Env var loading + Zod validation
├── types.ts                 # Shared app-level types
├── routes/
│   ├── stats.ts             # GET /:username
│   ├── streak.ts            # GET /:username/streak
│   ├── top-langs.ts         # GET /:username/top-langs
│   └── health.ts            # GET /health (used by Docker HEALTHCHECK)
├── fetchers/
│   ├── github.ts            # GitHub GraphQL client (single query, typed responses)
│   └── types.ts             # GitHub API response types
├── cards/
│   ├── base-card.ts         # Shared SVG utilities (dimensions, CSS, gradients, text escaping)
│   ├── stats-card.ts        # Stats card SVG builder
│   ├── streak-card.ts       # Streak card SVG builder
│   └── langs-card.ts        # Top languages card SVG builder
├── themes/
│   ├── index.ts             # Theme registry + lookup
│   └── themes.ts            # Built-in theme definitions
├── cache/
│   ├── index.ts             # Cache interface + factory (picks memory or redis based on env)
│   ├── memory.ts            # LRU in-memory cache (lru-cache package)
│   └── redis.ts             # Redis layer (enabled by REDIS_URL, provided by docker compose)
└── utils/
    ├── pat-pool.ts          # PAT token pool — round-robin rotation with retry on rate limit
    ├── sanitize.ts          # HTML/SVG entity escaping (encodeHTML)
    └── query-params.ts      # Zod schema for URL query parameters
```

### Docker files

```
Dockerfile               # Multi-stage: node:24-alpine, builder + runner, non-root user, HEALTHCHECK
docker-compose.yml       # app + redis services, healthchecks, restart policies, volumes
.dockerignore            # Excludes node_modules, .git, dist, .env
```

## Key patterns

### SVG generation

Cards are built as SVG XML strings using TypeScript template literals. Every card function returns a `string` containing a complete `<svg>` element.

- All user-provided text MUST be escaped via `encodeHTML()` from `utils/sanitize.ts`
- CSS animations go inside `<style>` tags within the `<svg>` — no external stylesheets
- No `<script>`, `onclick`, or `foreignObject` — GitHub's Camo proxy strips them
- Card dimensions should be hardcoded (e.g. 495×195 for stats, 495×195 for streak)

### GitHub API

- We use GitHub's **GraphQL API v4** exclusively — not the REST API
- Contribution calendar data is only available via GraphQL
- All queries go through `fetchers/github.ts` which handles: query building, PAT selection, fetch, error handling, response typing
- Response types live in `fetchers/types.ts`
- `fetch()` is native in Node.js 24 — do not import any HTTP client library

### PAT token pooling

- Env vars `PAT_1`, `PAT_2`, … `PAT_N` are read at startup
- `utils/pat-pool.ts` exports `getNextToken()` which round-robins
- On a 403 or rate-limit response, the fetcher calls `getNextToken()` and retries once
- Each PAT provides 5,000 GraphQL points/hour

### Caching

- Cache interface: `get(key): Promise<string | null>` and `set(key, value, ttlSeconds): Promise<void>`
- Cache keys follow the format: `card:{type}:{username}:{paramsHash}`
- The `paramsHash` is a deterministic hash of sorted, relevant query params
- In-memory cache: `lru-cache`, 500 max entries, default 4h TTL
- Redis cache: provided by the `redis` service in docker compose, connected via `REDIS_URL`

### HTTP responses

Every card endpoint returns:

```
Content-Type: image/svg+xml; charset=utf-8
Cache-Control: public, max-age=14400, s-maxage=14400, stale-while-revalidate=86400
ETag: <md5 of SVG body>
```

The `s-maxage` controls CDN caching (Cloudflare respects this). The `stale-while-revalidate` allows CDNs to serve stale content while revalidating in the background.

### Health check

`GET /health` returns `200 OK` with JSON status. This endpoint is used by:

- Docker `HEALTHCHECK` instruction in the Dockerfile
- docker compose healthcheck for the `app` service
- External uptime monitoring (e.g. Uptime Robot, Cloudflare health checks)

### Configuration

All config is loaded in `config.ts` using Zod for validation. The config object is created once at startup and passed to route handlers via Hono's context or module imports. Do not use `process.env` directly outside of `config.ts`.

## Development commands

```bash
pnpm dev            # Vite dev server with HMR
pnpm build          # Production build (Rolldown via Vite 8)
pnpm start          # Run production build: node dist/index.js
pnpm lint           # Oxlint
pnpm lint:fix       # Oxlint with auto-fix
pnpm format         # Oxfmt (write)
pnpm format:check   # Oxfmt (check only)
pnpm typecheck      # tsc --noEmit
pnpm check          # format:check + lint + typecheck
pnpm docker:build   # docker compose build
pnpm docker:up      # docker compose up -d
pnpm docker:down    # docker compose down
pnpm docker:logs    # docker compose logs -f app
```

## Version management

This project uses **pnpm's built-in Node.js management**. The `.npmrc` file pins the Node.js version via `use-node-version=24.13.1`. When you run any `pnpm` command, it automatically downloads and uses the correct Node.js version.

In Docker, we use `node:24-alpine` directly.

## Code style

- Oxfmt config: `printWidth: 100`, `singleQuote: true`, `trailingComma: "all"`, `semi: true`
- Import sorting is handled by Oxfmt (built-in) — do not add import sorting plugins
- Prefer `const` over `let`. Never use `var`.
- Prefer `function` declarations for top-level functions, arrow functions for callbacks
- Use explicit return types on exported functions
- Error handling: throw typed errors, catch in route handlers, return appropriate HTTP status
- No `any` — use `unknown` and narrow with type guards
- Prefer `interface` over `type` for object shapes (unless union/intersection is needed)

## Naming conventions

- Files: `kebab-case.ts`
- Variables/functions: `camelCase`
- Types/interfaces: `PascalCase`
- Constants: `UPPER_SNAKE_CASE` for true constants (config values, magic numbers), `camelCase` for derived values
- Card builder functions: `renderStatsCard(data, options): string`
- Route handlers: `statsRoute`, `streakRoute`, etc.

## Deployment

The service runs on a VPS inside Docker, behind nginx + Cloudflare CDN:

```
User (GitHub README) → Camo proxy → Cloudflare CDN → nginx → Docker (Node.js + Hono)
                                                                  ↕
                                                              Redis (docker compose)
```

Production deployment:

```bash
git pull
docker compose up -d --build
```

Docker handles:

- Process management (restart on crash via `restart: unless-stopped`)
- Health monitoring (via `HEALTHCHECK` instruction)
- Log aggregation (`docker compose logs -f app`)
- Resource isolation (non-root user in container)
- Redis lifecycle (separate container, persistent volume)

## Common tasks

### Adding a new card type

1. Create a fetcher function in `fetchers/github.ts` (or reuse the existing query)
2. Create `cards/new-card.ts` exporting `renderNewCard(data, options): string`
3. Create `routes/new.ts` with the Hono route handler
4. Register the route in `src/index.ts`
5. Add the card type to the cache key enum

### Adding a new theme

1. Add the theme object to `themes/themes.ts`
2. Register it in `themes/index.ts`
3. That's it — query param `?theme=your-theme` will work automatically

### Adding a new query parameter

1. Add the field to the Zod schema in `utils/query-params.ts`
2. Thread it through the route handler to the card builder
3. Make sure it's included in the cache key hash if it affects output

### Updating Node.js version

1. Update `use-node-version` in `.npmrc`
2. Update `devEngines.runtime.version` in root `package.json`
3. Update the `FROM node:XX-alpine` line in `Dockerfile`
4. Rebuild: `docker compose up -d --build`
