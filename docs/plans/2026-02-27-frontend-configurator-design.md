# Frontend Configurator + Two-Layer Cache Design

Date: 2026-02-27

## Problem

The current architecture caches rendered SVGs keyed by `username + optionsHash`. Any style change causes a cache miss and a new GitHub API call. With a frontend configurator where users tweak themes, colors, and layouts in real-time, this means every preview update burns a rate-limited API call — even though the underlying GitHub data hasn't changed.

Additionally, the project has no frontend. Users must manually construct query parameters to customize their cards.

## Goals

1. **Two-layer cache**: Separate GitHub data caching from SVG caching so style changes never trigger API calls.
2. **React configurator**: Interactive SPA with live preview, theme/color/option controls, and drag-and-drop card ordering.
3. **Embed output**: One-click copy of markdown/HTML embed code reflecting the user's configuration.

## Decisions

- **Frontend tech**: React SPA (Vite), in a top-level `frontend/` directory
- **Preview strategy**: Server-side rendering via `<img>` tags pointing at existing SVG endpoints (Approach A — single source of truth)
- **Drag-and-drop**: `@dnd-kit/core` + `@dnd-kit/sortable`
- **Data cache TTL**: 4 hours (same as current SVG cache)
- **Scope**: Full configurator with drag-and-drop in V1

## Two-Layer Cache

### Layer 1 — Data cache

- Key: `data:{username}`
- Value: serialized `FetchResult` JSON (stats + streak + languages)
- TTL: 4 hours
- One entry per username, shared across all card types
- Populated by GitHub API fetch, checked before making API calls

### Layer 2 — SVG cache

- Key: `svg:{type}:{username}:{optionsHash}`
- Value: rendered SVG string
- TTL: 4 hours
- On miss + data cache hit: render from cached data (no API call)
- On miss + data miss: fetch from GitHub, populate both caches

### Request flow

```
Request -> SVG cache hit? -> return SVG
                | miss
           Data cache hit? -> render SVG -> cache SVG -> return
                | miss
           GitHub API -> cache data -> render SVG -> cache SVG -> return
```

## New API Endpoint

`GET /api/data/:username` — Returns `FetchResult` as JSON.

Used by the frontend to validate the username and fetch the user's display name. Goes through the data cache. Not used for SVG rendering (the frontend uses `<img>` tags for that).

## Frontend Architecture

### Layout

```
+---------------------------------------------------+
|  GitCard                            [GitHub link]  |
+------------------+--------------------------------+
|                  |                                  |
|  Username input  |      Live Preview Area           |
|  -----------     |                                  |
|  Card selector   |  +---------------------------+  |
|  * Stats         |  |  Stats Card (draggable)   |  |
|  * Streak        |  +---------------------------+  |
|  * Top Langs     |  +---------------------------+  |
|  -----------     |  |  Streak Card (draggable)  |  |
|  Theme dropdown  |  +---------------------------+  |
|  Color pickers   |  +---------------------------+  |
|  Toggle options  |  |  Langs Card (draggable)   |  |
|  -----------     |  +---------------------------+  |
|  Hide stats      |                                  |
|  Locale          |                                  |
|                  |                                  |
+------------------+--------------------------------+
|  Copy Markdown    Copy HTML                        |
|                                                    |
|  ![Stats](https://domain/stats/user?theme=dark)   |
|  ![Streak](https://domain/stats/user/streak?...)   |
+----------------------------------------------------+
```

### Interactions

- Type username -> 300ms debounce -> cards load as `<img>` tags
- Toggle cards on/off -> add/remove from preview + embed output
- Drag-and-drop -> reorder cards -> embed output updates order
- Change theme/colors/options -> img src updates -> browser re-fetches SVG (cache-hot from data layer)
- Copy markdown/HTML -> clipboard with correct order and params

### Components

| Component           | Responsibility                         |
| ------------------- | -------------------------------------- |
| `App.tsx`           | Main layout, state coordination        |
| `CardPreview.tsx`   | `<img>` wrapper for a single SVG card  |
| `CardList.tsx`      | Drag-and-drop sortable container       |
| `Sidebar.tsx`       | All controls (left panel)              |
| `ThemePicker.tsx`   | Theme dropdown                         |
| `ColorPicker.tsx`   | Hex color inputs                       |
| `OptionToggles.tsx` | Boolean toggles (icons, border, title) |
| `EmbedOutput.tsx`   | Generated markdown/HTML + copy buttons |
| `useCardConfig.ts`  | Hook managing all configurator state   |

## Project Structure (new files)

```
frontend/
  index.html
  src/
    main.tsx
    App.tsx
    components/
      CardPreview.tsx
      CardList.tsx
      Sidebar.tsx
      ThemePicker.tsx
      ColorPicker.tsx
      OptionToggles.tsx
      EmbedOutput.tsx
    hooks/
      useCardConfig.ts
    styles/
      app.css
```

## Build Changes

- `frontend/` uses Vite client build, outputs to `dist/static/`
- Hono serves `dist/static/` for the SPA in production
- In dev, Vite dev server handles HMR for both server and client
- `vite.config.ts` updated for dual server + client builds

## New Dependencies

- `react`, `react-dom`
- `@dnd-kit/core`, `@dnd-kit/sortable`, `@dnd-kit/utilities`
- `@types/react`, `@types/react-dom` (dev)

## Existing Endpoints (unchanged)

- `GET /stats/:username` — SVG stats card
- `GET /stats/:username/streak` — SVG streak card
- `GET /stats/:username/top-langs` — SVG top languages card
- `GET /health` — health check
