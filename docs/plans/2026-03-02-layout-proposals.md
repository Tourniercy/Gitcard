# Layout Proposals Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement 3 alternative layout proposals (Hero+Tabs, Floating Toolbar, Top Controls Bar) as separate routes using TanStack Router, so the user can compare them side-by-side.

**Architecture:** Add TanStack Router (code-based routing, no file-based generation) to the existing web app. The current `App.tsx` becomes a root layout with a nav switcher. Each layout variant lives in its own route component. All three share the same hooks (`useCardConfig`, `useGitHubData`) and existing UI primitives (`CardList`, `CardPreview`, `ThemeCombobox`, etc.). A new `Sheet` UI component (shadcn) is needed for Layout B's side drawer.

**Tech Stack:** `@tanstack/react-router`, existing React 19 + Tailwind 4 + Radix UI + shadcn components

---

### Task 1: Install TanStack Router

**Files:**

- Modify: `apps/web/package.json`

**Step 1: Install the dependency**

Run:

```bash
cd apps/web && pnpm add @tanstack/react-router
```

**Step 2: Verify installation**

Run:

```bash
cd apps/web && pnpm ls @tanstack/react-router
```

Expected: Package listed with version

**Step 3: Commit**

```bash
git add apps/web/package.json apps/web/pnpm-lock.yaml pnpm-lock.yaml
git commit -m "chore(web): add @tanstack/react-router dependency"
```

---

### Task 2: Set up router with code-based routing

**Files:**

- Create: `apps/web/src/router.tsx`
- Modify: `apps/web/src/main.tsx`

**Step 1: Create the router file**

Create `apps/web/src/router.tsx` with root route, 4 child routes (`/` = current layout, `/layout-a` = Hero+Tabs, `/layout-b` = Floating Toolbar, `/layout-c` = Top Controls Bar), and placeholder components:

```tsx
import {
  Outlet,
  RouterProvider,
  Link,
  createRouter,
  createRoute,
  createRootRoute,
} from '@tanstack/react-router';
import { GitHubLogoIcon } from '@radix-ui/react-icons';
import { ModeToggle } from '@/components/mode-toggle';
import { Button } from '@/components/ui/button';

const rootRoute = createRootRoute({
  component: function RootLayout() {
    return (
      <div className="flex min-h-screen flex-col">
        <header className="sticky top-0 z-50 w-full border-b border-border/40 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
          <div className="container flex h-14 items-center">
            <a href="/" className="mr-2 flex items-center md:mr-6 md:space-x-2">
              <span className="font-bold">GitCard</span>
            </a>
            <nav className="flex flex-1 items-center gap-1">
              <Link
                to="/"
                className="rounded-md px-3 py-1.5 text-sm font-medium text-muted-foreground transition-colors hover:text-foreground [&.active]:bg-accent [&.active]:text-foreground"
                activeOptions={{ exact: true }}
              >
                Current
              </Link>
              <Link
                to="/layout-a"
                className="rounded-md px-3 py-1.5 text-sm font-medium text-muted-foreground transition-colors hover:text-foreground [&.active]:bg-accent [&.active]:text-foreground"
              >
                A: Hero + Tabs
              </Link>
              <Link
                to="/layout-b"
                className="rounded-md px-3 py-1.5 text-sm font-medium text-muted-foreground transition-colors hover:text-foreground [&.active]:bg-accent [&.active]:text-foreground"
              >
                B: Floating Toolbar
              </Link>
              <Link
                to="/layout-c"
                className="rounded-md px-3 py-1.5 text-sm font-medium text-muted-foreground transition-colors hover:text-foreground [&.active]:bg-accent [&.active]:text-foreground"
              >
                C: Top Controls
              </Link>
              <div className="ml-auto flex items-center gap-1">
                <Button variant="ghost" size="icon" className="size-8" asChild>
                  <a
                    aria-label="GitHub repo"
                    href="https://github.com/Tourniercy/Gitcard"
                    target="_blank"
                    rel="noopener noreferrer"
                  >
                    <GitHubLogoIcon className="size-4" aria-hidden="true" />
                  </a>
                </Button>
                <ModeToggle />
              </div>
            </nav>
          </div>
        </header>
        <Outlet />
      </div>
    );
  },
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: () => <div className="container py-6">Current layout placeholder</div>,
});

const layoutARoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/layout-a',
  component: () => <div className="container py-6">Layout A placeholder</div>,
});

const layoutBRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/layout-b',
  component: () => <div className="container py-6">Layout B placeholder</div>,
});

const layoutCRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/layout-c',
  component: () => <div className="container py-6">Layout C placeholder</div>,
});

const routeTree = rootRoute.addChildren([indexRoute, layoutARoute, layoutBRoute, layoutCRoute]);

export const router = createRouter({ routeTree });

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
```

**Step 2: Update main.tsx to use RouterProvider**

Replace the `<App />` render with `<RouterProvider router={router} />`:

```tsx
import * as Sentry from '@sentry/react';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';
import { RouterProvider } from '@tanstack/react-router';
import { ThemeProvider } from './components/theme-provider';
import { router } from './router';
import './index.css';

Sentry.init({
  dsn: import.meta.env.VITE_SENTRY_DSN,
  integrations: [Sentry.browserTracingIntegration(), Sentry.replayIntegration()],
  tracesSampleRate: 0.1,
  replaysSessionSampleRate: 0,
  replaysOnErrorSampleRate: 1.0,
});

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60 * 5,
      gcTime: 1000 * 60 * 30,
      retry: 1,
      refetchOnWindowFocus: false,
    },
  },
});

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Sentry.ErrorBoundary
      fallback={
        <div className="flex min-h-screen items-center justify-center">
          <p className="text-destructive">Something went wrong. Please refresh the page.</p>
        </div>
      }
    >
      <QueryClientProvider client={queryClient}>
        <ThemeProvider defaultTheme="system">
          <RouterProvider router={router} />
        </ThemeProvider>
        <ReactQueryDevtools initialIsOpen={false} />
      </QueryClientProvider>
    </Sentry.ErrorBoundary>
  </StrictMode>,
);
```

**Step 3: Verify it compiles**

Run:

```bash
cd apps/web && pnpm typecheck
```

Expected: No errors

**Step 4: Commit**

```bash
git add apps/web/src/router.tsx apps/web/src/main.tsx
git commit -m "feat(web): set up TanStack Router with code-based routing and layout nav"
```

---

### Task 3: Extract shared card state into a provider

Currently `useCardConfig` and `useGitHubData` are called in `App.tsx`. All 4 routes need the same state. Extract a shared context provider so all layouts share the same card configuration state.

**Files:**

- Create: `apps/web/src/components/CardConfigProvider.tsx`

**Step 1: Create the provider**

```tsx
import { createContext, useContext, useCallback } from 'react';
import { useDebounceValue } from 'usehooks-ts';
import { useCardConfig } from '@/hooks/useCardConfig';
import { useGitHubData } from '@/hooks/useGitHubData';
import type { CardConfig, CardType } from '@/hooks/useCardConfig';
import type { GitHubData } from '@/hooks/useGitHubData';
import type { CardOptions } from '@gitcard/svg-renderer';

const ALL_CARDS: CardType[] = ['stats', 'streak', 'top-langs', 'profile'];

interface CardConfigContextValue {
  config: CardConfig;
  setUsername: (username: string) => void;
  setTheme: (theme: string) => void;
  setOption: <K extends keyof CardConfig>(key: K, value: CardConfig[K]) => void;
  handleToggle: (key: string, value: boolean) => void;
  buildSrc: (card: CardType) => string;
  cardOptions: CardOptions;
  allCards: CardType[];
  data: GitHubData | undefined;
  isPending: boolean;
  error: Error | null;
}

const CardConfigContext = createContext<CardConfigContextValue | null>(null);

export function CardConfigProvider({ children }: { children: React.ReactNode }) {
  const { config, setUsername, setTheme, setOption, buildQueryString, cardPaths, cardOptions } =
    useCardConfig();

  const [debouncedUsername] = useDebounceValue(config.username.trim(), 300);
  const { data, isPending, error } = useGitHubData(debouncedUsername);

  const buildSrc = useCallback(
    (card: CardType): string => `${cardPaths[card]}${buildQueryString(card)}`,
    [cardPaths, buildQueryString],
  );

  const handleToggle = useCallback(
    (key: string, value: boolean) => {
      setOption(key as keyof CardConfig, value as CardConfig[keyof CardConfig]);
    },
    [setOption],
  );

  return (
    <CardConfigContext.Provider
      value={{
        config,
        setUsername,
        setTheme,
        setOption,
        handleToggle,
        buildSrc,
        cardOptions,
        allCards: ALL_CARDS,
        data,
        isPending,
        error,
      }}
    >
      {children}
    </CardConfigContext.Provider>
  );
}

export function useCardConfigContext(): CardConfigContextValue {
  const ctx = useContext(CardConfigContext);
  if (!ctx) throw new Error('useCardConfigContext must be used within CardConfigProvider');
  return ctx;
}
```

**Step 2: Wrap the root route component in the provider**

In `router.tsx`, import `CardConfigProvider` and wrap `<Outlet />` inside it:

```tsx
import { CardConfigProvider } from '@/components/CardConfigProvider';

// In rootRoute component:
<CardConfigProvider>
  <Outlet />
</CardConfigProvider>;
```

**Step 3: Verify compilation**

Run:

```bash
cd apps/web && pnpm typecheck
```

**Step 4: Commit**

```bash
git add apps/web/src/components/CardConfigProvider.tsx apps/web/src/router.tsx
git commit -m "feat(web): extract shared card config into context provider"
```

---

### Task 4: Extract reusable card preview area component

All 3 layouts need the same card preview area (loading/error/empty states + CardList). Extract it to a shared component.

**Files:**

- Create: `apps/web/src/components/CardPreviewArea.tsx`

**Step 1: Create the component**

```tsx
import { useCardConfigContext } from '@/components/CardConfigProvider';
import { CardList } from '@/components/CardList';

interface CardPreviewAreaProps {
  className?: string;
  gridClassName?: string;
}

export function CardPreviewArea({ className, gridClassName }: CardPreviewAreaProps) {
  const { config, data, isPending, error, allCards, cardOptions, buildSrc } =
    useCardConfigContext();

  if (!config.username.trim()) {
    return (
      <div className={className}>
        <div className="flex min-h-[300px] items-center justify-center rounded-lg border-2 border-dashed">
          <p className="text-muted-foreground">Enter a GitHub username to get started</p>
        </div>
      </div>
    );
  }

  if (isPending) {
    return (
      <div className={className}>
        <div className="flex min-h-[300px] items-center justify-center rounded-lg border-2 border-dashed">
          <div className="flex flex-col items-center gap-2">
            <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent" />
            <p className="text-sm text-muted-foreground">Loading...</p>
          </div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className={className}>
        <div className="flex min-h-[300px] items-center justify-center rounded-lg border-2 border-dashed border-destructive bg-destructive/5">
          <p className="text-destructive">{error.message}</p>
        </div>
      </div>
    );
  }

  if (!data) return null;

  return (
    <div className={className}>
      <CardList
        cards={allCards}
        data={data}
        options={cardOptions}
        buildSrc={buildSrc}
        gridClassName={gridClassName}
      />
    </div>
  );
}
```

**Step 2: Update CardList to accept gridClassName prop**

In `CardList.tsx`, add an optional `gridClassName` prop that defaults to the current `flex flex-col gap-4`:

```tsx
interface CardListProps {
  cards: CardType[];
  data: GitHubData;
  options: CardOptions;
  buildSrc: (card: CardType) => string;
  gridClassName?: string;
}

export function CardList({ cards, data, options, buildSrc, gridClassName }: CardListProps) {
  return (
    <div className={gridClassName ?? 'flex flex-col gap-4'}>{/* ...existing card map... */}</div>
  );
}
```

**Step 3: Verify**

Run:

```bash
cd apps/web && pnpm typecheck
```

**Step 4: Commit**

```bash
git add apps/web/src/components/CardPreviewArea.tsx apps/web/src/components/CardList.tsx
git commit -m "feat(web): extract reusable CardPreviewArea and add gridClassName to CardList"
```

---

### Task 5: Wire up "/" route with current sidebar layout

**Files:**

- Modify: `apps/web/src/router.tsx`

**Step 1: Implement the index route**

Replace the placeholder for `indexRoute` with the current sidebar layout, using `useCardConfigContext`:

```tsx
import { Sidebar } from '@/components/Sidebar';
import { CardPreviewArea } from '@/components/CardPreviewArea';
import { useCardConfigContext } from '@/components/CardConfigProvider';

// indexRoute component:
function CurrentLayout() {
  const { config, setUsername, setTheme, handleToggle, setOption } = useCardConfigContext();

  return (
    <div className="container grid flex-1 grid-cols-1 gap-6 py-6 md:grid-cols-[300px_1fr]">
      <Sidebar
        config={config}
        onUsernameChange={setUsername}
        onThemeChange={setTheme}
        onToggle={handleToggle}
        onOptionChange={setOption}
      />
      <CardPreviewArea className="flex min-w-0 flex-col gap-6" />
    </div>
  );
}
```

**Step 2: Verify it works**

Run:

```bash
cd apps/web && pnpm dev
```

Open `http://localhost:5173/` — should look identical to the current layout.

**Step 3: Commit**

```bash
git add apps/web/src/router.tsx
git commit -m "feat(web): wire current sidebar layout to index route"
```

---

### Task 6: Implement Layout A — "Hero + Tabs"

Single-column layout. Username input at top (hero). Controls in collapsible tab sections (Theme, Options, Colors). Cards in a responsive 2-column grid below.

**Files:**

- Create: `apps/web/src/components/layouts/LayoutA.tsx`
- Modify: `apps/web/src/router.tsx`

**Step 1: Create Layout A component**

```tsx
import { useState } from 'react';
import { ChevronDown } from 'lucide-react';
import { useCardConfigContext } from '@/components/CardConfigProvider';
import { CardPreviewArea } from '@/components/CardPreviewArea';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Checkbox } from '@/components/ui/checkbox';
import { ThemeCombobox } from '@/components/ThemeCombobox';
import { LocaleCombobox } from '@/components/LocaleCombobox';
import { ColorInput } from '@/components/ColorInput';
import { Button } from '@/components/ui/button';

function CollapsibleSection({
  title,
  defaultOpen = false,
  children,
}: {
  title: string;
  defaultOpen?: boolean;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <div className="rounded-lg border bg-card">
      <button
        type="button"
        className="flex w-full items-center justify-between p-3 text-sm font-medium"
        onClick={() => setOpen(!open)}
      >
        {title}
        <ChevronDown className={`h-4 w-4 transition-transform ${open ? 'rotate-180' : ''}`} />
      </button>
      {open && <div className="border-t px-3 pb-3 pt-2">{children}</div>}
    </div>
  );
}

export function LayoutA() {
  const { config, setUsername, setTheme, setOption, handleToggle } = useCardConfigContext();

  return (
    <div className="container flex flex-col gap-6 py-8">
      {/* Hero: Username input */}
      <div className="mx-auto w-full max-w-lg">
        <Label htmlFor="username-hero" className="sr-only">
          GitHub Username
        </Label>
        <Input
          id="username-hero"
          value={config.username}
          onChange={(e) => setUsername(e.target.value)}
          placeholder="Enter a GitHub username..."
          className="h-12 text-center text-lg"
          autoComplete="off"
          spellCheck={false}
        />
      </div>

      {/* Inline controls row */}
      <div className="flex flex-wrap items-end justify-center gap-3">
        <div className="w-48">
          <Label className="mb-1 text-xs text-muted-foreground">Theme</Label>
          <ThemeCombobox value={config.theme} onValueChange={setTheme} />
        </div>
        <div className="w-40">
          <Label className="mb-1 text-xs text-muted-foreground">Locale</Label>
          <LocaleCombobox value={config.locale} onValueChange={(v) => setOption('locale', v)} />
        </div>
      </div>

      {/* Collapsible sections */}
      <div className="mx-auto grid w-full max-w-2xl gap-3 sm:grid-cols-2">
        <CollapsibleSection title="Options">
          <div className="flex flex-col gap-3">
            <div className="flex items-center justify-between">
              <Label htmlFor="a-show-icons" className="text-sm font-normal">
                Show icons
              </Label>
              <Switch
                id="a-show-icons"
                checked={config.showIcons}
                onCheckedChange={(v) => handleToggle('showIcons', v)}
              />
            </div>
            <div className="flex items-center justify-between">
              <Label htmlFor="a-hide-border" className="text-sm font-normal">
                Hide border
              </Label>
              <Switch
                id="a-hide-border"
                checked={config.hideBorder}
                onCheckedChange={(v) => handleToggle('hideBorder', v)}
              />
            </div>
            <div className="flex items-center justify-between">
              <Label htmlFor="a-hide-title" className="text-sm font-normal">
                Hide title
              </Label>
              <Switch
                id="a-hide-title"
                checked={config.hideTitle}
                onCheckedChange={(v) => handleToggle('hideTitle', v)}
              />
            </div>
          </div>
        </CollapsibleSection>

        <CollapsibleSection title="Hide Stats">
          <div className="flex flex-wrap gap-x-3 gap-y-2">
            {(['stars', 'commits', 'prs', 'issues', 'forks', 'contribs'] as const).map((stat) => (
              <div key={stat} className="flex items-center gap-1.5">
                <Checkbox
                  id={`a-hide-${stat}`}
                  checked={config.hide.includes(stat)}
                  onCheckedChange={(checked) => {
                    const next = checked
                      ? [...config.hide, stat]
                      : config.hide.filter((s) => s !== stat);
                    setOption('hide', next);
                  }}
                />
                <Label htmlFor={`a-hide-${stat}`} className="text-sm font-normal">
                  {stat}
                </Label>
              </div>
            ))}
          </div>
        </CollapsibleSection>

        <CollapsibleSection title="Custom Colors" defaultOpen={false}>
          <div className="flex flex-col gap-2">
            {(
              [
                ['bgColor', 'Background'],
                ['titleColor', 'Title'],
                ['textColor', 'Text'],
                ['iconColor', 'Icon'],
                ['borderColor', 'Border'],
              ] as const
            ).map(([key, label]) => (
              <ColorInput
                key={key}
                id={`a-color-${key}`}
                label={label}
                value={config[key]}
                onChange={(v) => setOption(key, v)}
              />
            ))}
          </div>
        </CollapsibleSection>
      </div>

      {/* Cards in 2-column grid */}
      <CardPreviewArea className="w-full" gridClassName="grid grid-cols-1 gap-4 lg:grid-cols-2" />
    </div>
  );
}
```

**Step 2: Wire to route in router.tsx**

```tsx
import { LayoutA } from '@/components/layouts/LayoutA';

const layoutARoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/layout-a',
  component: LayoutA,
});
```

**Step 3: Verify**

Run:

```bash
cd apps/web && pnpm dev
```

Open `http://localhost:5173/layout-a` — Hero input, inline theme/locale, collapsible options, 2-col cards.

**Step 4: Commit**

```bash
git add apps/web/src/components/layouts/LayoutA.tsx apps/web/src/router.tsx
git commit -m "feat(web): implement Layout A — Hero + Tabs with collapsible sections"
```

---

### Task 7: Add Sheet UI component (needed for Layout B)

Layout B uses a slide-out sheet/drawer for advanced settings. Add the shadcn Sheet component.

**Files:**

- Create: `apps/web/src/components/ui/sheet.tsx`

**Step 1: Create the Sheet component**

Use the shadcn Sheet component based on Radix Dialog primitive. The component should provide `Sheet`, `SheetTrigger`, `SheetContent`, `SheetHeader`, `SheetTitle`, `SheetDescription`, `SheetFooter`, `SheetClose`. Content slides in from the right with a backdrop overlay.

Build it from the `Dialog` primitive in `radix-ui` (same package already installed), adding side-based positioning (right/left/top/bottom) via `cva` variants and slide-in/out animations.

**Step 2: Verify**

Run:

```bash
cd apps/web && pnpm typecheck
```

**Step 3: Commit**

```bash
git add apps/web/src/components/ui/sheet.tsx
git commit -m "feat(web): add Sheet UI component for Layout B"
```

---

### Task 8: Implement Layout B — "Floating Toolbar"

Minimal toolbar (username + theme + locale + "Settings" button). Settings button opens a Sheet from the right containing Options, Colors, and Hide Stats in tabs. Cards are the hero in a 2-column grid.

**Files:**

- Create: `apps/web/src/components/layouts/LayoutB.tsx`
- Modify: `apps/web/src/router.tsx`

**Step 1: Create Layout B component**

```tsx
import { useCardConfigContext } from '@/components/CardConfigProvider';
import { CardPreviewArea } from '@/components/CardPreviewArea';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Checkbox } from '@/components/ui/checkbox';
import { Separator } from '@/components/ui/separator';
import { ThemeCombobox } from '@/components/ThemeCombobox';
import { LocaleCombobox } from '@/components/LocaleCombobox';
import { ColorInput } from '@/components/ColorInput';
import { Button } from '@/components/ui/button';
import { Settings } from 'lucide-react';
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetTrigger } from '@/components/ui/sheet';

export function LayoutB() {
  const { config, setUsername, setTheme, setOption, handleToggle } = useCardConfigContext();

  return (
    <div className="container flex flex-col gap-6 py-6">
      {/* Floating toolbar */}
      <div className="flex flex-wrap items-end gap-3 rounded-lg border bg-card p-3 shadow-sm">
        <div className="flex-1 min-w-[200px]">
          <Input
            value={config.username}
            onChange={(e) => setUsername(e.target.value)}
            placeholder="GitHub username..."
            autoComplete="off"
            spellCheck={false}
          />
        </div>
        <div className="w-44">
          <ThemeCombobox value={config.theme} onValueChange={setTheme} />
        </div>
        <div className="w-36">
          <LocaleCombobox value={config.locale} onValueChange={(v) => setOption('locale', v)} />
        </div>
        <Sheet>
          <SheetTrigger asChild>
            <Button variant="outline" size="icon">
              <Settings className="h-4 w-4" />
              <span className="sr-only">Settings</span>
            </Button>
          </SheetTrigger>
          <SheetContent>
            <SheetHeader>
              <SheetTitle>Card Settings</SheetTitle>
            </SheetHeader>
            <div className="flex flex-col gap-4 py-4">
              {/* Options */}
              <div className="flex flex-col gap-3">
                <Label className="font-medium">Options</Label>
                <div className="flex items-center justify-between">
                  <Label htmlFor="b-show-icons" className="text-sm font-normal">
                    Show icons
                  </Label>
                  <Switch
                    id="b-show-icons"
                    checked={config.showIcons}
                    onCheckedChange={(v) => handleToggle('showIcons', v)}
                  />
                </div>
                <div className="flex items-center justify-between">
                  <Label htmlFor="b-hide-border" className="text-sm font-normal">
                    Hide border
                  </Label>
                  <Switch
                    id="b-hide-border"
                    checked={config.hideBorder}
                    onCheckedChange={(v) => handleToggle('hideBorder', v)}
                  />
                </div>
                <div className="flex items-center justify-between">
                  <Label htmlFor="b-hide-title" className="text-sm font-normal">
                    Hide title
                  </Label>
                  <Switch
                    id="b-hide-title"
                    checked={config.hideTitle}
                    onCheckedChange={(v) => handleToggle('hideTitle', v)}
                  />
                </div>
              </div>

              <Separator />

              {/* Hide Stats */}
              <div className="flex flex-col gap-3">
                <Label className="font-medium">Hide Stats</Label>
                <div className="flex flex-wrap gap-x-3 gap-y-2">
                  {(['stars', 'commits', 'prs', 'issues', 'forks', 'contribs'] as const).map(
                    (stat) => (
                      <div key={stat} className="flex items-center gap-1.5">
                        <Checkbox
                          id={`b-hide-${stat}`}
                          checked={config.hide.includes(stat)}
                          onCheckedChange={(checked) => {
                            const next = checked
                              ? [...config.hide, stat]
                              : config.hide.filter((s) => s !== stat);
                            setOption('hide', next);
                          }}
                        />
                        <Label htmlFor={`b-hide-${stat}`} className="text-sm font-normal">
                          {stat}
                        </Label>
                      </div>
                    ),
                  )}
                </div>
              </div>

              <Separator />

              {/* Custom Colors */}
              <div className="flex flex-col gap-3">
                <Label className="font-medium">Custom Colors</Label>
                {(
                  [
                    ['bgColor', 'Background'],
                    ['titleColor', 'Title'],
                    ['textColor', 'Text'],
                    ['iconColor', 'Icon'],
                    ['borderColor', 'Border'],
                  ] as const
                ).map(([key, label]) => (
                  <ColorInput
                    key={key}
                    id={`b-color-${key}`}
                    label={label}
                    value={config[key]}
                    onChange={(v) => setOption(key, v)}
                  />
                ))}
              </div>
            </div>
          </SheetContent>
        </Sheet>
      </div>

      {/* Cards as hero — 2-column grid */}
      <CardPreviewArea className="w-full" gridClassName="grid grid-cols-1 gap-4 lg:grid-cols-2" />
    </div>
  );
}
```

**Step 2: Wire to route**

```tsx
import { LayoutB } from '@/components/layouts/LayoutB';
```

**Step 3: Verify**

Open `http://localhost:5173/layout-b` — toolbar at top, settings sheet slides from right, cards below.

**Step 4: Commit**

```bash
git add apps/web/src/components/layouts/LayoutB.tsx apps/web/src/router.tsx
git commit -m "feat(web): implement Layout B — Floating Toolbar with settings sheet"
```

---

### Task 9: Implement Layout C — "Top Controls Bar"

Horizontal control strip with popover buttons. Full-width stacked cards below.

**Files:**

- Create: `apps/web/src/components/layouts/LayoutC.tsx`
- Modify: `apps/web/src/router.tsx`

**Step 1: Create Layout C component**

```tsx
import { useCardConfigContext } from '@/components/CardConfigProvider';
import { CardPreviewArea } from '@/components/CardPreviewArea';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Checkbox } from '@/components/ui/checkbox';
import { ThemeCombobox } from '@/components/ThemeCombobox';
import { LocaleCombobox } from '@/components/LocaleCombobox';
import { ColorInput } from '@/components/ColorInput';
import { Button } from '@/components/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { Sliders, Palette, BarChart3 } from 'lucide-react';

export function LayoutC() {
  const { config, setUsername, setTheme, setOption, handleToggle } = useCardConfigContext();

  return (
    <div className="container flex flex-col gap-6 py-6">
      {/* Username */}
      <div className="max-w-md">
        <Input
          value={config.username}
          onChange={(e) => setUsername(e.target.value)}
          placeholder="GitHub username..."
          autoComplete="off"
          spellCheck={false}
        />
      </div>

      {/* Controls bar */}
      <div className="flex flex-wrap items-center gap-2">
        <div className="w-44">
          <ThemeCombobox value={config.theme} onValueChange={setTheme} />
        </div>
        <div className="w-36">
          <LocaleCombobox value={config.locale} onValueChange={(v) => setOption('locale', v)} />
        </div>

        {/* Options popover */}
        <Popover>
          <PopoverTrigger asChild>
            <Button variant="outline" size="sm" className="gap-1.5">
              <Sliders className="h-3.5 w-3.5" />
              Options
            </Button>
          </PopoverTrigger>
          <PopoverContent className="w-56" align="start">
            <div className="flex flex-col gap-3">
              <div className="flex items-center justify-between">
                <Label htmlFor="c-show-icons" className="text-sm font-normal">
                  Show icons
                </Label>
                <Switch
                  id="c-show-icons"
                  checked={config.showIcons}
                  onCheckedChange={(v) => handleToggle('showIcons', v)}
                />
              </div>
              <div className="flex items-center justify-between">
                <Label htmlFor="c-hide-border" className="text-sm font-normal">
                  Hide border
                </Label>
                <Switch
                  id="c-hide-border"
                  checked={config.hideBorder}
                  onCheckedChange={(v) => handleToggle('hideBorder', v)}
                />
              </div>
              <div className="flex items-center justify-between">
                <Label htmlFor="c-hide-title" className="text-sm font-normal">
                  Hide title
                </Label>
                <Switch
                  id="c-hide-title"
                  checked={config.hideTitle}
                  onCheckedChange={(v) => handleToggle('hideTitle', v)}
                />
              </div>
            </div>
          </PopoverContent>
        </Popover>

        {/* Colors popover */}
        <Popover>
          <PopoverTrigger asChild>
            <Button variant="outline" size="sm" className="gap-1.5">
              <Palette className="h-3.5 w-3.5" />
              Colors
            </Button>
          </PopoverTrigger>
          <PopoverContent className="w-64" align="start">
            <div className="flex flex-col gap-2">
              {(
                [
                  ['bgColor', 'Background'],
                  ['titleColor', 'Title'],
                  ['textColor', 'Text'],
                  ['iconColor', 'Icon'],
                  ['borderColor', 'Border'],
                ] as const
              ).map(([key, label]) => (
                <ColorInput
                  key={key}
                  id={`c-color-${key}`}
                  label={label}
                  value={config[key]}
                  onChange={(v) => setOption(key, v)}
                />
              ))}
            </div>
          </PopoverContent>
        </Popover>

        {/* Stats popover */}
        <Popover>
          <PopoverTrigger asChild>
            <Button variant="outline" size="sm" className="gap-1.5">
              <BarChart3 className="h-3.5 w-3.5" />
              Stats
            </Button>
          </PopoverTrigger>
          <PopoverContent className="w-56" align="start">
            <div className="flex flex-wrap gap-x-3 gap-y-2">
              {(['stars', 'commits', 'prs', 'issues', 'forks', 'contribs'] as const).map((stat) => (
                <div key={stat} className="flex items-center gap-1.5">
                  <Checkbox
                    id={`c-hide-${stat}`}
                    checked={config.hide.includes(stat)}
                    onCheckedChange={(checked) => {
                      const next = checked
                        ? [...config.hide, stat]
                        : config.hide.filter((s) => s !== stat);
                      setOption('hide', next);
                    }}
                  />
                  <Label htmlFor={`c-hide-${stat}`} className="text-sm font-normal">
                    {stat}
                  </Label>
                </div>
              ))}
            </div>
          </PopoverContent>
        </Popover>
      </div>

      {/* Full-width stacked cards */}
      <CardPreviewArea className="w-full" />
    </div>
  );
}
```

**Step 2: Wire to route**

```tsx
import { LayoutC } from '@/components/layouts/LayoutC';
```

**Step 3: Verify**

Open `http://localhost:5173/layout-c` — compact toolbar, popover buttons, full-width cards.

**Step 4: Commit**

```bash
git add apps/web/src/components/layouts/LayoutC.tsx apps/web/src/router.tsx
git commit -m "feat(web): implement Layout C — Top Controls Bar with popovers"
```

---

### Task 10: Final verification and cleanup

**Step 1: Type check**

Run:

```bash
cd apps/web && pnpm typecheck
```

Expected: No errors

**Step 2: Dev server test**

Run:

```bash
cd apps/web && pnpm dev
```

Verify all 4 routes work:

- `http://localhost:5173/` — Current sidebar layout
- `http://localhost:5173/layout-a` — Hero + Tabs
- `http://localhost:5173/layout-b` — Floating Toolbar
- `http://localhost:5173/layout-c` — Top Controls Bar

**Step 3: Navigation test**

Click each nav link in the header. Verify:

- Active link is highlighted
- Card config state persists across route changes (same username, same theme)
- Cards render correctly in all layouts

**Step 4: Commit**

```bash
git add -A
git commit -m "feat(web): complete layout proposal comparison with TanStack Router"
```
