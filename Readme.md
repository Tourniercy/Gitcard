# github-stats-cards

A self-hosted service that dynamically generates SVG stat cards from GitHub user data. Built with Hono, TypeScript, and the VoidZero toolchain (Vite 8 + Rolldown + Oxlint + Oxfmt).

Inspired by [github-readme-stats](https://github.com/anuraghazra/github-readme-stats) and [github-readme-streak-stats](https://github.com/DenverCoder1/github-readme-streak-stats).

## Usage

Embed in your GitHub profile README:

```md
![Stats](https://your-domain.com/anuraghazra)
![Streak](https://your-domain.com/anuraghazra/streak)
![Top Langs](https://your-domain.com/anuraghazra/top-langs)
```

### Query parameters

| Parameter       | Type    | Default     | Description                                                    |
| --------------- | ------- | ----------- | -------------------------------------------------------------- |
| `theme`         | string  | `"default"` | Built-in theme name                                            |
| `hide`          | string  | —           | Comma-separated stats to hide (e.g. `stars,commits`)           |
| `show_icons`    | boolean | `true`      | Show icons next to stats                                       |
| `hide_border`   | boolean | `false`     | Remove card border                                             |
| `hide_title`    | boolean | `false`     | Remove card title                                              |
| `bg_color`      | string  | —           | Background hex (e.g. `0d1117`) or gradient (`angle,start,end`) |
| `title_color`   | string  | —           | Title hex color                                                |
| `text_color`    | string  | —           | Body text hex color                                            |
| `icon_color`    | string  | —           | Icon hex color                                                 |
| `border_color`  | string  | —           | Border hex color                                               |
| `cache_seconds` | number  | `14400`     | Cache TTL (min 14400, max 86400)                               |
| `locale`        | string  | `"en"`      | Locale for number formatting                                   |

Example with parameters:

```
https://your-domain.com/anuraghazra?theme=tokyonight&hide=issues&show_icons=true
```

### Available themes

`default` · `dark` · `radical` · `tokyonight` · `gruvbox` · `dracula` · `nord`

## Tech stack

|                 |                                                                                       |
| --------------- | ------------------------------------------------------------------------------------- |
| Runtime         | Node.js 24 LTS (Krypton)                                                              |
| Framework       | [Hono](https://hono.dev) + [@hono/node-server](https://github.com/honojs/node-server) |
| Language        | TypeScript 5.8+ (strict)                                                              |
| Build           | [Vite 8](https://vite.dev) (Rolldown-powered)                                         |
| Linter          | [Oxlint](https://oxc.rs/docs/guide/usage/linter)                                      |
| Formatter       | [Oxfmt](https://oxc.rs/docs/guide/usage/formatter)                                    |
| Version manager | [Volta](https://volta.sh) (local dev)                                                 |
| Package manager | pnpm                                                                                  |
| Deployment      | Docker                                                                                |

## Architecture

```
Request → CDN (Cloudflare) → VPS (Docker → Node.js + Hono)
                                ├── In-memory LRU cache (lru-cache)
                                ├── Redis cache (via docker compose)
                                └── GitHub GraphQL API (PAT pool)
                                        ↓
                                  SVG string response
                                  + Cache-Control headers
```

The service fetches GitHub user data via the GraphQL API, generates SVG cards using TypeScript template literals, and returns them with aggressive cache headers. A CDN sits in front to serve cached responses at the edge.

### PAT token pooling

Each GitHub Personal Access Token provides 5,000 GraphQL points/hour. The service reads `PAT_1` through `PAT_N` from environment variables and rotates between them automatically. With 8 tokens you get 40,000 requests/hour before caching even kicks in.

### Caching layers

1. **CDN edge** — `Cache-Control: public, s-maxage=14400` (4 hours) serves most requests without hitting origin
2. **In-memory LRU** — 500 entries, 4-hour TTL, near-zero latency for cache hits
3. **Redis** — runs alongside the app via docker compose, survives container restarts

## Getting started

### Prerequisites

- [Volta](https://volta.sh) — manages Node.js and pnpm versions automatically
- [Docker](https://docs.docker.com/get-docker/) + [Docker Compose](https://docs.docker.com/compose/)
- A GitHub Personal Access Token ([create one](https://github.com/settings/tokens) — no special scopes needed)

### Install Volta

```bash
curl https://get.volta.sh | bash
```

Volta's pnpm support is experimental. Add this to your shell profile (`~/.zshrc`, `~/.bashrc`, etc.):

```bash
export VOLTA_FEATURE_PNPM=1
```

Then install the pinned tools:

```bash
volta install node@24
volta install pnpm@10
```

### Setup

```bash
git clone https://github.com/your-user/github-stats-cards.git
cd github-stats-cards
pnpm install
cp .env.example .env
```

Volta reads the `volta` section in `package.json` and automatically activates Node 24 and pnpm 10 when you enter the project directory. No manual switching.

Edit `.env` and add at least one PAT:

```env
PAT_1=ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

### Development

```bash
pnpm dev
```

Open [http://localhost:3000/your-github-username](http://localhost:3000/your-github-username).

Vite 8 provides HMR — edit any file and the server reloads instantly.

### Build & run (local, without Docker)

```bash
pnpm build
pnpm start
```

### Docker (recommended for production)

```bash
# Build and start everything (app + Redis)
docker compose up -d

# View logs
docker compose logs -f app

# Rebuild after code changes
docker compose up -d --build

# Stop
docker compose down
```

The Docker image uses `node:24-alpine` and a multi-stage build for a minimal production image. Health checks are built into both the Dockerfile and docker-compose.yml.

## Deploying to a VPS

### 1. Provision a server

Any $4–5/month VPS works (Hetzner, DigitalOcean, Vultr). Install Docker:

```bash
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER
```

### 2. Deploy

```bash
git clone https://github.com/your-user/github-stats-cards.git
cd github-stats-cards
cp .env.example .env
# Edit .env with your PATs
docker compose up -d
```

That's it. Docker handles process management, restarts, and health monitoring. No PM2, no systemd service files.

### 3. Reverse proxy (nginx)

```nginx
server {
    listen 80;
    server_name stats.your-domain.com;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

Or skip nginx entirely and let Cloudflare connect directly to port 3000 via a Cloudflare Tunnel.

### 4. CDN (Cloudflare)

Point your domain's DNS to the VPS through Cloudflare (orange cloud). Cloudflare will:

- Cache SVG responses at 300+ edge locations based on `Cache-Control` headers
- Provide free SSL
- Absorb traffic spikes

Set Cloudflare's **Browser Cache TTL** to "Respect Existing Headers" so it uses the 4-hour `s-maxage` from the response.

> **GitHub Camo proxy**: All images in GitHub READMEs pass through `camo.githubusercontent.com`, which caches aggressively (~31 days). Even with a CDN, README viewers may see stale cards. The service sets `ETag` headers to help with conditional requests.

### Updating in production

```bash
git pull
docker compose up -d --build
```

Docker compose rebuilds the image and performs a rolling restart. Zero-downtime if you add a second replica.

## Scripts

| Script              | Description                            |
| ------------------- | -------------------------------------- |
| `pnpm dev`          | Start Vite dev server with HMR         |
| `pnpm build`        | Production build via Vite 8 (Rolldown) |
| `pnpm start`        | Run the production build               |
| `pnpm preview`      | Build + start in one command           |
| `pnpm lint`         | Run Oxlint                             |
| `pnpm lint:fix`     | Run Oxlint with auto-fix               |
| `pnpm format`       | Format all files with Oxfmt            |
| `pnpm format:check` | Check formatting without writing       |
| `pnpm typecheck`    | Run `tsc --noEmit`                     |
| `pnpm check`        | Run format:check + lint + typecheck    |
| `pnpm docker:build` | Build Docker image                     |
| `pnpm docker:up`    | Start containers (detached)            |
| `pnpm docker:down`  | Stop containers                        |
| `pnpm docker:logs`  | Tail app container logs                |

## Configuration

All configuration is via environment variables (see `.env.example`):

| Variable          | Required         | Default              | Description                                                   |
| ----------------- | ---------------- | -------------------- | ------------------------------------------------------------- |
| `PAT_1` … `PAT_N` | Yes (at least 1) | —                    | GitHub Personal Access Tokens for API access                  |
| `PORT`            | No               | `3000`               | Server port                                                   |
| `REDIS_URL`       | No               | `redis://redis:6379` | Redis connection URL (docker compose sets this automatically) |
| `CACHE_TTL`       | No               | `14400`              | Default cache TTL in seconds (4 hours)                        |
| `LOG_LEVEL`       | No               | `info`               | Log level (`debug`, `info`, `warn`, `error`)                  |

## Contributing

### Setup

1. Install [Volta](https://volta.sh) and enable pnpm support (`VOLTA_FEATURE_PNPM=1`)
2. Clone the repo and run `pnpm install` — Volta handles Node/pnpm versions automatically
3. Copy `.env.example` to `.env` and add a GitHub PAT
4. Run `pnpm dev`

### Before pushing

```bash
pnpm check
```

The project uses Oxlint (not ESLint) and Oxfmt (not Prettier). If your editor supports the [Oxc VS Code extension](https://marketplace.visualstudio.com/items?itemName=oxc.oxc-vscode), enable it for inline lint + format-on-save.

## License

MIT
