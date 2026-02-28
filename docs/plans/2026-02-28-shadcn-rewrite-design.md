# shadcn/ui Rewrite — Web App Design

## Goal

Replace the vanilla CSS frontend with shadcn/ui components, Tailwind CSS v4, and a tabbed sidebar layout. Add system-aware dark mode.

## Tech Stack Changes

**Add:**

- `tailwindcss` + `@tailwindcss/vite` (Tailwind CSS v4)
- `tw-animate-css` (shadcn animations)
- `lucide-react` (icon library)
- `class-variance-authority` + `clsx` + `tailwind-merge` (shadcn utility deps)
- shadcn components: Button, Input, Select, Checkbox, Tabs, Card, Label, Separator, Badge, Tooltip, Switch, DropdownMenu

**Remove:**

- `apps/web/src/styles/app.css` (replaced by Tailwind + shadcn CSS vars)

**Keep:**

- `@dnd-kit/*` (drag-and-drop)
- `@gitcard/svg-renderer` (shared SVG renderer)
- React 19, Vite 8 beta

## Setup

- `@tailwindcss/vite` plugin in vite.config.ts
- `@` path alias via `resolve.alias`
- `components.json` for shadcn CLI (style: new-york, rsc: false, base color: neutral)
- CSS variables in `src/index.css` for light/dark theming (oklch color space)
- `ThemeProvider` context wrapping app in `main.tsx`
- tsconfig `paths` alias: `"@/*": ["./src/*"]`

## Layout

### App Shell

- Sticky header: "GitCard" title + subtitle + theme toggle (Sun/Moon `DropdownMenu`)
- CSS Grid body: sidebar (300px) + main content (1fr)
- Mobile (< 768px): single column, sidebar stacks on top

### Sidebar (3 tabs)

| Tab           | Controls                                                                                                |
| ------------- | ------------------------------------------------------------------------------------------------------- |
| **Configure** | Username `Input`, card selection `Checkbox`es, theme `Select`, option `Switch`es (icons, border, title) |
| **Colors**    | 5 color pickers: native `<input type="color">` + hex `Input` + clear `Button`                           |
| **Export**    | Markdown + HTML embed code blocks with copy `Button`s                                                   |

### Main Content

- Empty: muted prompt text
- Loading: `animate-pulse` skeleton
- Error: destructive text
- Preview: drag-and-drop `CardList`, each card in a shadcn `Card` with drag handle

## Component Mapping

| Current File        | New Implementation                                        |
| ------------------- | --------------------------------------------------------- |
| `Sidebar.tsx`       | Tabbed wrapper with `Tabs`/`TabsList`/`TabsContent`       |
| `ThemePicker.tsx`   | Inline in Configure tab — shadcn `Select`                 |
| `OptionToggles.tsx` | Inline in Configure tab — shadcn `Switch` + `Label`       |
| `ColorPicker.tsx`   | Retained — native color input + shadcn `Input` + `Button` |
| `CardPreview.tsx`   | Same render logic, wrapped in shadcn `Card`               |
| `CardList.tsx`      | Same dnd-kit logic, Tailwind styling                      |
| `EmbedOutput.tsx`   | Moved into Export tab in sidebar                          |
| `App.tsx`           | Same hooks, new Tailwind layout                           |

## New Files

| File                                | Purpose                                    |
| ----------------------------------- | ------------------------------------------ |
| `components.json`                   | shadcn CLI config                          |
| `src/index.css`                     | Tailwind imports + CSS theme variables     |
| `src/lib/utils.ts`                  | `cn()` helper (clsx + tailwind-merge)      |
| `src/components/ui/*.tsx`           | shadcn component files (installed via CLI) |
| `src/components/theme-provider.tsx` | Dark mode context provider                 |
| `src/components/mode-toggle.tsx`    | Light/dark/system dropdown                 |

## Decisions

- **Tailwind v4** (not v3) — uses `@import "tailwindcss"` and `@theme inline`, no config file
- **new-york style** — shadcn's more compact style variant
- **neutral base color** — clean gray palette that works in both modes
- **No Tailwind config file** — v4 handles everything via CSS
- **Export tab in sidebar** — consolidates all output in one place instead of below cards
