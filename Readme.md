<div align="center">
  <h1>GitCard</h1>
  <p>Dynamically generated GitHub stats cards for your profile README.</p>

  <p>
    <a href="https://github.com/Tourniercy/Gitcard/issues"><img src="https://img.shields.io/github/issues/Tourniercy/Gitcard?color=0088ff" alt="Issues"></a>
    <a href="https://github.com/Tourniercy/Gitcard/pulls"><img src="https://img.shields.io/github/issues-pr/Tourniercy/Gitcard?color=0088ff" alt="Pull Requests"></a>
    <a href="https://github.com/Tourniercy/Gitcard/blob/main/LICENSE"><img src="https://img.shields.io/github/license/Tourniercy/Gitcard" alt="License"></a>
  </p>

  <p>
    <a href="https://gitcard.rilcy.app">View Demo</a>
    &middot;
    <a href="https://github.com/Tourniercy/Gitcard/issues/new">Report Bug</a>
    &middot;
    <a href="https://github.com/Tourniercy/Gitcard/issues/new">Request Feature</a>
  </p>

  <br />
  <p><strong><a href="https://gitcard.rilcy.app/">Create your cards visually</a></strong> — pick a theme, customize colors, and copy the embed code.</p>
</div>

---

## Stats Card

![Tourniercy's GitHub stats](https://api-gitcard.rilcy.app/stats/tourniercy)

Copy and paste this into your README to get started:

```md
[![Tourniercy's GitHub stats](https://api-gitcard.rilcy.app/stats/tourniercy)](https://github.com/Tourniercy/Gitcard)
```

### Hiding individual stats

You can hide specific stat items with the `hide` parameter.

```md
![Stats](https://api-gitcard.rilcy.app/stats/tourniercy?hide=stars,forks)
```

> [!NOTE]
> Available values: `stars`, `commits`, `prs`, `issues`, `forks`, `contributed`

### Showing icons

Icons are displayed by default. You can turn them off:

```md
![Stats](https://api-gitcard.rilcy.app/stats/tourniercy?show_icons=false)
```

### Themes

Use any built-in theme via the `theme` parameter:

```md
![Stats](https://api-gitcard.rilcy.app/stats/tourniercy?theme=tokyo-night)
```

![Stats with Tokyo Night theme](https://api-gitcard.rilcy.app/stats/tourniercy?theme=tokyo-night)

<details>
<summary>All available themes</summary>

#### Light

`default` `flat` `breezy` `rose` `mint` `sand` `lavender` `cotton-candy` `paper`

#### Dark

`dark` `dim` `midnight` `onyx`

#### Editor

`dracula` `monokai` `nord` `solarized` `solarized-dark` `gruvbox` `gruvbox-light` `tokyo-night` `one-dark` `one-light` `catppuccin-mocha` `catppuccin-latte` `synthwave` `cobalt` `night-owl`

#### Atmospheric

`aurora` `ocean` `forest` `sunset` `cherry` `slate` `high-contrast`

</details>

#### Responsive theme (dark/light mode)

Use GitHub's `#gh-dark-mode-only` and `#gh-light-mode-only` fragments to show different themes based on the viewer's GitHub theme:

```html
<picture>
  <source
    media="(prefers-color-scheme: dark)"
    srcset="https://api-gitcard.rilcy.app/stats/tourniercy?theme=tokyo-night"
  />
  <source
    media="(prefers-color-scheme: light)"
    srcset="https://api-gitcard.rilcy.app/stats/tourniercy?theme=default"
  />
  <img alt="GitHub Stats" src="https://api-gitcard.rilcy.app/stats/tourniercy" />
</picture>
```

### Customization

| Parameter      | Type    | Default   | Description                                                                                 |
| -------------- | ------- | --------- | ------------------------------------------------------------------------------------------- |
| `theme`        | string  | `default` | Built-in theme name                                                                         |
| `hide`         | string  | -         | Comma-separated stats to hide (`stars`, `commits`, `prs`, `issues`, `forks`, `contributed`) |
| `show_icons`   | boolean | `true`    | Show icons next to stats                                                                    |
| `hide_border`  | boolean | `false`   | Remove card border                                                                          |
| `hide_title`   | boolean | `false`   | Remove card title                                                                           |
| `bg_color`     | string  | -         | Background hex (e.g. `0d1117`) or gradient (`angle,start,end`)                              |
| `title_color`  | string  | -         | Title hex color                                                                             |
| `text_color`   | string  | -         | Body text hex color                                                                         |
| `icon_color`   | string  | -         | Icon hex color                                                                              |
| `border_color` | string  | -         | Border hex color                                                                            |
| `locale`       | string  | `en`      | Locale for number formatting                                                                |

---

## Streak Card

![Tourniercy's streak](https://api-gitcard.rilcy.app/stats/tourniercy/streak)

```md
[![Tourniercy's streak](https://api-gitcard.rilcy.app/stats/tourniercy/streak)](https://github.com/Tourniercy/Gitcard)
```

Shows your current streak, longest streak, and total contributions. Supports the same `theme`, `hide_border`, and color parameters as the stats card.

---

## Top Languages Card

![Tourniercy's top languages](https://api-gitcard.rilcy.app/stats/tourniercy/top-langs)

```md
[![Top Langs](https://api-gitcard.rilcy.app/stats/tourniercy/top-langs)](https://github.com/Tourniercy/Gitcard)
```

Displays your most-used languages across public repositories, calculated by code size. Supports the same `theme`, `hide_border`, and color parameters as the stats card.

---

## All Demos

![Stats](https://api-gitcard.rilcy.app/stats/tourniercy?theme=default)

![Streak](https://api-gitcard.rilcy.app/stats/tourniercy/streak?theme=default)

![Top Langs](https://api-gitcard.rilcy.app/stats/tourniercy/top-langs?theme=default)

### Quick Tip: Align Cards Side by Side

```html
<a href="https://github.com/Tourniercy/Gitcard">
  <img src="https://api-gitcard.rilcy.app/stats/tourniercy" width="400" />
  <img src="https://api-gitcard.rilcy.app/stats/tourniercy/streak" width="400" />
</a>
```

---

## Deploy Your Own

GitCard is designed to be self-hosted. No shared public instance, no rate-limit headaches, your own tokens, your own rules.

### Prerequisites

- [Docker](https://docs.docker.com/get-docker/) + [Docker Compose](https://docs.docker.com/compose/)
- A GitHub Personal Access Token ([create one](https://github.com/settings/tokens) -- no special scopes needed)

### Quick Start

```bash
git clone https://github.com/Tourniercy/Gitcard.git
cd Gitcard
cp apps/api/.env.example apps/api/.env
```

Edit `apps/api/.env` and add at least one PAT:

```env
PAT_1=ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

Then start everything:

```bash
docker compose up -d
```

That's it. Open `http://localhost:3000/stats/tourniercy` and you should see an SVG card.

### Environment Variables

| Variable            | Required         | Default              | Description                                                   |
| ------------------- | ---------------- | -------------------- | ------------------------------------------------------------- |
| `PAT_1` ... `PAT_N` | Yes (at least 1) | -                    | GitHub Personal Access Tokens for API access                  |
| `PORT`              | No               | `3000`               | Server port                                                   |
| `REDIS_URL`         | No               | `redis://redis:6379` | Redis connection URL (docker compose sets this automatically) |
| `CACHE_TTL`         | No               | `14400`              | Cache TTL in seconds (4 hours)                                |
| `LOG_LEVEL`         | No               | `info`               | Log level (`debug`, `info`, `warn`, `error`)                  |
| `CORS_ORIGIN`       | No               | `*`                  | Allowed CORS origin for the web frontend                      |

### PAT Token Pooling

Each GitHub Personal Access Token provides 5,000 GraphQL points/hour. GitCard reads `PAT_1` through `PAT_N` from environment variables and rotates between them automatically. With 8 tokens you get 40,000 requests/hour before caching even kicks in.

### Production Setup (VPS + CDN)

Any $4-5/month VPS works (Hetzner, DigitalOcean, Vultr).

```
User (GitHub README) -> Camo proxy -> Cloudflare CDN -> nginx -> Docker (Node.js + Hono)
                                                                    |
                                                                  Redis
```

1. **Install Docker** on your VPS
2. **Clone and deploy**: `git clone`, configure `.env`, `docker compose up -d`
3. **Reverse proxy**: Point nginx (or Cloudflare Tunnel) to port 3000
4. **CDN**: Put Cloudflare in front -- it respects the `Cache-Control: s-maxage=14400` headers automatically

Docker handles process management, restarts, health monitoring, and log aggregation. No PM2, no systemd.

> [!NOTE]
> GitHub's Camo proxy caches images aggressively (~31 days). Even with a CDN, README viewers may see stale cards. The service sets `ETag` headers to help with conditional requests.

### Updating

```bash
git pull
docker compose up -d --build
```

---

## Architecture

```
Request -> CDN (Cloudflare) -> VPS (Docker -> Node.js + Hono)
                                  |-- In-memory LRU cache (lru-cache)
                                  |-- Redis cache (via docker compose)
                                  '-- GitHub GraphQL API (PAT pool)
                                          |
                                    SVG string response
                                    + Cache-Control headers
```

### Caching Layers

1. **CDN edge** -- `Cache-Control: public, s-maxage=14400` (4 hours) serves most requests without hitting origin
2. **In-memory LRU** -- 500 entries, 4-hour TTL, near-zero latency for cache hits
3. **Redis** -- runs alongside the app via docker compose, survives container restarts

### Tech Stack

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

---

## Contributing

### Local Development Setup

1. Install [pnpm](https://pnpm.io/installation) (v10+)
2. Clone the repo and run `pnpm install` -- pnpm auto-downloads the correct Node.js version
3. Copy `.env.example` to `.env` in both `apps/api/` and `apps/web/` and add a GitHub PAT
4. Run `pnpm dev`

### Scripts

| Script              | Description                            |
| ------------------- | -------------------------------------- |
| `pnpm dev`          | Start Vite dev server with HMR         |
| `pnpm build`        | Production build via Vite 8 (Rolldown) |
| `pnpm start`        | Run the production build               |
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

### Before Pushing

```bash
pnpm check
```

The project uses Oxlint (not ESLint) and Oxfmt (not Prettier). If your editor supports the [Oxc VS Code extension](https://marketplace.visualstudio.com/items?itemName=oxc.oxc-vscode), enable it for inline lint + format-on-save.

---

## License

MIT

---

<div align="center">
  Made with TypeScript and the <a href="https://voidzero.dev">VoidZero</a> toolchain.
</div>
