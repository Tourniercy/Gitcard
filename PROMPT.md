# Prompt: Generate a GitHub Stats Card Service Template

Use this prompt with Claude (or any LLM with computer use) to scaffold the project.

---

## Prompt

You are scaffolding a production-ready **GitHub Stats Card Service** — a self-hosted API that dynamically generates SVG (and optionally PNG) stat cards from GitHub user data, similar to `github-readme-stats` and `github-readme-streak-stats`.

### Target deployment

- **VPS** (e.g. Hetzner, DigitalOcean) running **Docker**
- **Cloudflare CDN** (or any reverse proxy CDN) in front for edge caching
- **Docker-only** — no PM2, no systemd, no bare-metal node process. The container IS the process manager.
- Multi-stage Dockerfile for minimal image size

### Runtime version management

- Use **Volta** to pin Node.js and pnpm versions for local development
- `package.json` must include a `"volta"` section pinning `node@24` and `pnpm`
- The Dockerfile uses the official `node:24-alpine` image directly (Volta is for local dev only)
- Do NOT use nvm, fnm, n, or any other version manager

### Tech stack (strict — do not substitute)

| Layer               | Technology                          | Notes                                                                                                              |
| ------------------- | ----------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| **Runtime**         | Node.js 24 LTS (Krypton, 24.13.x)   | V8 13.6, Undici 7, global URLPattern, native fetch                                                                 |
| **Framework**       | Hono (`hono` + `@hono/node-server`) | Ultralight, Web Standards based                                                                                    |
| **Language**        | TypeScript 5.8+ (strict mode)       |                                                                                                                    |
| **Build**           | Vite 8.0.0-beta.15 + Rolldown       | Use `@hono/vite-build/node` and `@hono/vite-dev-server` with `nodeAdapter`                                         |
| **Linter**          | Oxlint 1.x                          | Config: `.oxlintrc.json` with `correctness` + `suspicious` categories, `typescript` + `import` + `unicorn` plugins |
| **Formatter**       | Oxfmt (beta)                        | Config: `.oxfmtrc.json`, Prettier-compatible defaults, `printWidth: 100`                                           |
| **Package manager** | pnpm 10+                            |                                                                                                                    |
| **Version manager** | Volta (local dev)                   | Pins node + pnpm in package.json                                                                                   |
| **Container**       | Docker + docker compose             | Multi-stage build, node:24-alpine                                                                                  |

### Documentation lookup — always use Context7

Before writing or generating any code, **always use Context7 MCP** to fetch up-to-date documentation for every library and framework in this project. Append `use context7` to any doc lookup, or call `resolve-library-id` → `get-library-docs` directly.

This is critical because several tools in this stack are pre-1.0 or in beta (Vite 8, Oxfmt, Rolldown). Training data is likely outdated. Context7 pulls version-specific docs from source.

Libraries to check via Context7 before generating code:

| Library                      | Why                                                                         |
| ---------------------------- | --------------------------------------------------------------------------- |
| `hono`                       | Routing API, middleware, `c.header()`, `c.body()`                           |
| `@hono/node-server`          | `serve()` options, graceful shutdown                                        |
| `@hono/vite-build`           | Build plugin config for Node target                                         |
| `@hono/vite-dev-server`      | Dev server + `nodeAdapter` setup                                            |
| `vite`                       | Vite 8 beta config — `build.rolldownOptions` replaces `build.rollupOptions` |
| `zod`                        | Schema API for query param validation                                       |
| `lru-cache`                  | Constructor options, TTL, max entries                                       |
| `ioredis` / `@upstash/redis` | Connection, get/set with TTL                                                |
| `oxlint`                     | `.oxlintrc.json` schema, plugin names, category names                       |
| `oxfmt`                      | `.oxfmtrc.json` schema, import sorting config                               |

**Do not guess APIs. Do not rely on training data for these libraries. Always verify with Context7 first.**

### Architecture to implement

```
src/
├── index.ts                 # Hono app entry + @hono/node-server serve()
├── routes/
│   ├── stats.ts             # GET /:username — main stats card
│   ├── streak.ts            # GET /:username/streak — streak card
│   ├── top-langs.ts         # GET /:username/top-langs — top languages card
│   └── health.ts            # GET /health — healthcheck
├── fetchers/
│   ├── github.ts            # GitHub GraphQL API client
│   └── types.ts             # GitHub API response types
├── cards/
│   ├── stats-card.ts        # Stats card SVG builder
│   ├── streak-card.ts       # Streak card SVG builder
│   ├── langs-card.ts        # Top langs SVG builder
│   └── base-card.ts         # Shared SVG utilities (dimensions, CSS, gradients, escaping)
├── themes/
│   ├── index.ts             # Theme registry
│   └── themes.ts            # Built-in themes (dark, light, radical, tokyonight, etc.)
├── cache/
│   ├── index.ts             # Cache interface + factory
│   ├── memory.ts            # LRU in-memory cache (lru-cache)
│   └── redis.ts             # Optional Redis/Upstash layer (ioredis or @upstash/redis)
├── utils/
│   ├── pat-pool.ts          # PAT token pooling + rotation (PAT_1..PAT_N env vars)
│   ├── sanitize.ts          # HTML/SVG entity escaping
│   └── query-params.ts      # Zod schema for URL query params (theme, hide, show_icons, etc.)
├── config.ts                # Environment config with validation (zod)
└── types.ts                 # Shared app types
```

### Key implementation details

1. **SVG Generation**: Use TypeScript template literals to build SVG strings. Embed CSS `<style>` blocks inside `<svg>` for animations. The `base-card.ts` should handle:
   - Card dimensions, padding, borders, border-radius
   - CSS injection (animations, fonts via `@import`)
   - Gradient/pattern backgrounds
   - `encodeHTML()` for all user-provided text

2. **GitHub GraphQL API** (`fetchers/github.ts`):
   - Single query fetching: `user.contributionsCollection` (with `contributionCalendar`), `repositories` (first 100, sorted by stars), `followers.totalCount`
   - Use `fetch()` — native in Node.js 24, no polyfill or library needed
   - Return strongly typed results

3. **PAT Token Pooling** (`utils/pat-pool.ts`):
   - Read `PAT_1` through `PAT_N` from environment
   - Round-robin rotation
   - On 403/rate-limit response, skip to next token and retry (max 1 retry per token)
   - Log which token is active + remaining rate limit from response headers

4. **Caching** (`cache/`):
   - Interface: `get(key): Promise<string | null>`, `set(key, value, ttlSeconds): Promise<void>`
   - Memory layer: `lru-cache` with 500 entry max, 4-hour TTL default
   - Redis layer: optional, enabled via `REDIS_URL` env var
   - Cache key format: `card:{type}:{username}:{paramsHash}`

5. **HTTP Response Headers** (set in each route):

   ```
   Content-Type: image/svg+xml; charset=utf-8
   Cache-Control: public, max-age=14400, s-maxage=14400, stale-while-revalidate=86400
   ETag: <md5 of SVG content>
   ```

6. **Query Parameters** (validated via Zod):
   - `theme` (string, default: "default")
   - `hide` (comma-separated stat keys to hide)
   - `show_icons` (boolean, default: true)
   - `hide_border` (boolean, default: false)
   - `hide_title` (boolean, default: false)
   - `bg_color` (hex without #, or gradient like `30,e96443,904e95`)
   - `title_color`, `text_color`, `icon_color`, `border_color` (hex without #)
   - `cache_seconds` (number, min: 14400, max: 86400)
   - `locale` (string, default: "en")

7. **Themes** (`themes/`):
   - Each theme is an object: `{ bg_color, title_color, text_color, icon_color, border_color }`
   - Include at least: `default`, `dark`, `radical`, `tokyonight`, `gruvbox`, `dracula`, `nord`
   - Query param colors override theme colors

8. **Health check** (`routes/health.ts`):
   - `GET /health` returns `200 OK` with JSON: `{ status: "ok", uptime, cache: "memory"|"redis", tokens: <count> }`
   - Used by Docker `HEALTHCHECK` and monitoring

### Config files to generate

- `package.json` — with `type: "module"`, `volta` section, all scripts
- `tsconfig.json` — strict, `target: "ES2024"`, `module: "ESNext"`, `moduleResolution: "bundler"`, `jsx: "react-jsx"`, `jsxImportSource: "hono/jsx"`
- `vite.config.ts` — using `@hono/vite-build/node` + `@hono/vite-dev-server` with `nodeAdapter`
- `.oxlintrc.json` — with `$schema`, categories `correctness` + `suspicious` enabled, plugins `typescript`, `import`, `unicorn`, `oxc`
- `.oxfmtrc.json` — with `$schema`, `printWidth: 100`, `singleQuote: true`, `trailingComma: "all"`, `semi: true`, import sorting enabled
- `.env.example` — with all env vars documented
- `Dockerfile` — multi-stage (builder + runner), `node:24-alpine`, non-root user, `HEALTHCHECK`
- `docker-compose.yml` — app + optional Redis, env_file, restart policy, healthcheck
- `.dockerignore`
- `.gitignore`

### Volta section in package.json

```json
{
  "volta": {
    "node": "24.13.1",
    "pnpm": "10.5.2"
  }
}
```

Anyone with Volta installed automatically gets the correct Node.js and pnpm when entering the project directory. No manual version switching.

> **Note**: Volta's pnpm support is experimental. Contributors need `VOLTA_FEATURE_PNPM=1` in their shell profile. Document this in the README.

### Dockerfile

```dockerfile
FROM node:24-alpine AS base

FROM base AS builder
RUN corepack enable && corepack prepare pnpm@10 --activate
WORKDIR /app
COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY . .
RUN pnpm build

FROM base AS runner
RUN addgroup --system --gid 1001 app && \
    adduser --system --uid 1001 app
WORKDIR /app
COPY --from=builder --chown=app:app /app/dist ./dist
COPY --from=builder --chown=app:app /app/node_modules ./node_modules
COPY --from=builder --chown=app:app /app/package.json ./
USER app
EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://localhost:3000/health || exit 1
CMD ["node", "dist/index.js"]
```

### docker-compose.yml

```yaml
services:
  app:
    build: .
    ports:
      - '${PORT:-3000}:3000'
    env_file: .env
    restart: unless-stopped
    depends_on:
      redis:
        condition: service_healthy
    healthcheck:
      test: ['CMD', 'wget', '--no-verbose', '--tries=1', '--spider', 'http://localhost:3000/health']
      interval: 30s
      timeout: 3s
      retries: 3
      start_period: 5s

  redis:
    image: redis:7-alpine
    volumes:
      - redis-data:/data
    restart: unless-stopped
    healthcheck:
      test: ['CMD', 'redis-cli', 'ping']
      interval: 10s
      timeout: 3s
      retries: 3

volumes:
  redis-data:
```

### Scripts in package.json

```json
{
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "start": "node dist/index.js",
    "preview": "vite build && node dist/index.js",
    "lint": "oxlint .",
    "lint:fix": "oxlint --fix .",
    "format": "oxfmt",
    "format:check": "oxfmt --check",
    "typecheck": "tsc --noEmit",
    "check": "pnpm format:check && pnpm lint && pnpm typecheck",
    "docker:build": "docker compose build",
    "docker:up": "docker compose up -d",
    "docker:down": "docker compose down",
    "docker:logs": "docker compose logs -f app"
  }
}
```

### What NOT to include

- No frontend / client-side code (this is an API-only service)
- No database (caching only)
- No authentication for consumers (the API is public)
- No tests yet (will add later)
- No CI/CD pipeline yet (will add later)
- No PM2, no systemd, no process manager — Docker handles restarts and health
- Do not use Express, Fastify, or any other framework
- Do not use Prettier, ESLint, or Biome
- Do not use esbuild or Rollup directly (Vite 8 uses Rolldown internally)
- Do not use axios or node-fetch (use native `fetch`)
- Do not use nvm, fnm, or n — Volta is the version manager

### Output

Generate every file listed above with real, working implementation code. Not stubs, not TODOs — actual working code that I can `pnpm install && pnpm dev` and hit `http://localhost:3000/anuraghazra` to get an SVG card.

The SVG cards should look clean and professional, similar to the reference projects. Include at least basic stats: total stars, total commits, total PRs, total issues, and total contributions.

**REMINDER: Before writing ANY code, use Context7 to fetch current docs for every library you're about to use. Do not rely on training data. `use context7`**
