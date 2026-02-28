# shadcn/ui Web App Rewrite — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the vanilla CSS web frontend with shadcn/ui components, Tailwind CSS v4, tabbed sidebar, and system-aware dark mode.

**Architecture:** Keep all existing hooks and business logic unchanged. Replace CSS classes with Tailwind utility classes and swap HTML elements for shadcn components. Add ThemeProvider for dark mode. Restructure sidebar into 3 tabs (Configure, Colors, Export).

**Tech Stack:** Tailwind CSS v4, shadcn/ui (new-york style), lucide-react icons, class-variance-authority + clsx + tailwind-merge

---

## Task 1: Install Tailwind CSS v4 and configure Vite

**Files:**

- Modify: `apps/web/package.json`
- Modify: `apps/web/vite.config.ts`
- Modify: `apps/web/tsconfig.json`
- Create: `apps/web/src/index.css`
- Delete: `apps/web/src/styles/app.css`

**Step 1: Install Tailwind CSS v4 + Vite plugin + shadcn utility deps**

```bash
cd apps/web
pnpm add tailwindcss @tailwindcss/vite tw-animate-css class-variance-authority clsx tailwind-merge lucide-react
```

**Step 2: Add `@tailwindcss/vite` plugin and `@` alias to vite.config.ts**

```ts
import path from 'path';
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    port: 5173,
    proxy: {
      '/api': 'http://localhost:3001',
      '/stats': 'http://localhost:3001',
      '/health': 'http://localhost:3001',
    },
  },
});
```

**Step 3: Add path alias to tsconfig.json**

Add `"paths"` to compilerOptions:

```json
{
  "extends": "../../tsconfig.base.json",
  "compilerOptions": {
    "jsx": "react-jsx",
    "outDir": "dist",
    "rootDir": "src",
    "types": ["react", "react-dom"],
    "baseUrl": ".",
    "paths": {
      "@/*": ["./src/*"]
    }
  },
  "include": ["src/**/*.ts", "src/**/*.tsx"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

**Step 4: Create `src/index.css` with Tailwind + shadcn theme variables**

Replace the old `styles/app.css` import with this file. Full CSS with light/dark theme variables (neutral base color, oklch color space):

```css
@import 'tailwindcss';
@import 'tw-animate-css';

@custom-variant dark (&:is(.dark *));

:root {
  --background: oklch(1 0 0);
  --foreground: oklch(0.145 0 0);
  --card: oklch(1 0 0);
  --card-foreground: oklch(0.145 0 0);
  --popover: oklch(1 0 0);
  --popover-foreground: oklch(0.145 0 0);
  --primary: oklch(0.205 0 0);
  --primary-foreground: oklch(0.985 0 0);
  --secondary: oklch(0.97 0 0);
  --secondary-foreground: oklch(0.205 0 0);
  --muted: oklch(0.97 0 0);
  --muted-foreground: oklch(0.556 0 0);
  --accent: oklch(0.97 0 0);
  --accent-foreground: oklch(0.205 0 0);
  --destructive: oklch(0.577 0.245 27.325);
  --destructive-foreground: oklch(0.577 0.245 27.325);
  --border: oklch(0.922 0 0);
  --input: oklch(0.922 0 0);
  --ring: oklch(0.708 0 0);
  --chart-1: oklch(0.646 0.222 41.116);
  --chart-2: oklch(0.6 0.118 184.704);
  --chart-3: oklch(0.398 0.07 227.392);
  --chart-4: oklch(0.828 0.189 84.429);
  --chart-5: oklch(0.769 0.188 70.08);
  --radius: 0.625rem;
  --sidebar: oklch(0.985 0 0);
  --sidebar-foreground: oklch(0.145 0 0);
  --sidebar-primary: oklch(0.205 0 0);
  --sidebar-primary-foreground: oklch(0.985 0 0);
  --sidebar-accent: oklch(0.97 0 0);
  --sidebar-accent-foreground: oklch(0.205 0 0);
  --sidebar-border: oklch(0.922 0 0);
  --sidebar-ring: oklch(0.708 0 0);
}

.dark {
  --background: oklch(0.145 0 0);
  --foreground: oklch(0.985 0 0);
  --card: oklch(0.145 0 0);
  --card-foreground: oklch(0.985 0 0);
  --popover: oklch(0.145 0 0);
  --popover-foreground: oklch(0.985 0 0);
  --primary: oklch(0.985 0 0);
  --primary-foreground: oklch(0.205 0 0);
  --secondary: oklch(0.269 0 0);
  --secondary-foreground: oklch(0.985 0 0);
  --muted: oklch(0.269 0 0);
  --muted-foreground: oklch(0.708 0 0);
  --accent: oklch(0.269 0 0);
  --accent-foreground: oklch(0.985 0 0);
  --destructive: oklch(0.396 0.141 25.723);
  --destructive-foreground: oklch(0.637 0.237 25.331);
  --border: oklch(0.269 0 0);
  --input: oklch(0.269 0 0);
  --ring: oklch(0.439 0 0);
  --chart-1: oklch(0.488 0.243 264.376);
  --chart-2: oklch(0.696 0.17 162.48);
  --chart-3: oklch(0.769 0.188 70.08);
  --chart-4: oklch(0.627 0.265 303.9);
  --chart-5: oklch(0.645 0.246 16.439);
  --sidebar: oklch(0.205 0 0);
  --sidebar-foreground: oklch(0.985 0 0);
  --sidebar-primary: oklch(0.488 0.243 264.376);
  --sidebar-primary-foreground: oklch(0.985 0 0);
  --sidebar-accent: oklch(0.269 0 0);
  --sidebar-accent-foreground: oklch(0.985 0 0);
  --sidebar-border: oklch(0.269 0 0);
  --sidebar-ring: oklch(0.439 0 0);
}

@theme inline {
  --color-background: var(--background);
  --color-foreground: var(--foreground);
  --color-card: var(--card);
  --color-card-foreground: var(--card-foreground);
  --color-popover: var(--popover);
  --color-popover-foreground: var(--popover-foreground);
  --color-primary: var(--primary);
  --color-primary-foreground: var(--primary-foreground);
  --color-secondary: var(--secondary);
  --color-secondary-foreground: var(--secondary-foreground);
  --color-muted: var(--muted);
  --color-muted-foreground: var(--muted-foreground);
  --color-accent: var(--accent);
  --color-accent-foreground: var(--accent-foreground);
  --color-destructive: var(--destructive);
  --color-destructive-foreground: var(--destructive-foreground);
  --color-border: var(--border);
  --color-input: var(--input);
  --color-ring: var(--ring);
  --color-chart-1: var(--chart-1);
  --color-chart-2: var(--chart-2);
  --color-chart-3: var(--chart-3);
  --color-chart-4: var(--chart-4);
  --color-chart-5: var(--chart-5);
  --radius-sm: calc(var(--radius) - 4px);
  --radius-md: calc(var(--radius) - 2px);
  --radius-lg: var(--radius);
  --radius-xl: calc(var(--radius) + 4px);
  --color-sidebar: var(--sidebar);
  --color-sidebar-foreground: var(--sidebar-foreground);
  --color-sidebar-primary: var(--sidebar-primary);
  --color-sidebar-primary-foreground: var(--sidebar-primary-foreground);
  --color-sidebar-accent: var(--sidebar-accent);
  --color-sidebar-accent-foreground: var(--sidebar-accent-foreground);
  --color-sidebar-border: var(--sidebar-border);
  --color-sidebar-ring: var(--sidebar-ring);
}

@layer base {
  * {
    @apply border-border outline-ring/50;
  }
  body {
    @apply bg-background text-foreground;
  }
}
```

**Step 5: Delete `apps/web/src/styles/app.css`**

**Step 6: Update `main.tsx` to import `./index.css` instead of old styles**

In `main.tsx`, there is no direct CSS import (App.tsx imports it). The CSS import change happens in App.tsx (Task 6).

**Step 7: Verify build works**

```bash
cd /d/Projets/gitcard && pnpm install && pnpm --filter web build
```

Expected: Build succeeds (components will be unstyled but no errors).

**Step 8: Commit**

```bash
git add apps/web/
git commit -m "chore(web): add Tailwind CSS v4, remove vanilla CSS"
```

---

## Task 2: Create shadcn infrastructure (`components.json`, `cn()` utility)

**Files:**

- Create: `apps/web/components.json`
- Create: `apps/web/src/lib/utils.ts`

**Step 1: Create `components.json` for shadcn CLI**

```json
{
  "$schema": "https://ui.shadcn.com/schema.json",
  "style": "new-york",
  "rsc": false,
  "tsx": true,
  "tailwind": {
    "config": "",
    "css": "src/index.css",
    "baseColor": "neutral",
    "cssVariables": true,
    "prefix": ""
  },
  "aliases": {
    "components": "@/components",
    "utils": "@/lib/utils",
    "ui": "@/components/ui",
    "lib": "@/lib",
    "hooks": "@/hooks"
  },
  "iconLibrary": "lucide"
}
```

**Step 2: Create `src/lib/utils.ts`**

```ts
import { type ClassValue, clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}
```

**Step 3: Commit**

```bash
git add apps/web/components.json apps/web/src/lib/utils.ts
git commit -m "chore(web): add shadcn components.json and cn() utility"
```

---

## Task 3: Install shadcn UI components

**Files:**

- Create: `apps/web/src/components/ui/button.tsx`
- Create: `apps/web/src/components/ui/input.tsx`
- Create: `apps/web/src/components/ui/label.tsx`
- Create: `apps/web/src/components/ui/select.tsx`
- Create: `apps/web/src/components/ui/checkbox.tsx`
- Create: `apps/web/src/components/ui/switch.tsx`
- Create: `apps/web/src/components/ui/tabs.tsx`
- Create: `apps/web/src/components/ui/card.tsx`
- Create: `apps/web/src/components/ui/separator.tsx`
- Create: `apps/web/src/components/ui/badge.tsx`
- Create: `apps/web/src/components/ui/tooltip.tsx`
- Create: `apps/web/src/components/ui/dropdown-menu.tsx`

**Step 1: Install all components via shadcn CLI**

Run from `apps/web/`:

```bash
cd apps/web
npx shadcn@latest add button input label select checkbox switch tabs card separator badge tooltip dropdown-menu
```

If the CLI doesn't work with Vite 8 beta, install manually by copying component source from shadcn docs. The CLI creates files in `src/components/ui/`.

**Step 2: Verify typecheck**

```bash
cd /d/Projets/gitcard && pnpm --filter web typecheck
```

**Step 3: Commit**

```bash
git add apps/web/src/components/ui/
git commit -m "chore(web): install shadcn UI components"
```

---

## Task 4: Add ThemeProvider and ModeToggle

**Files:**

- Create: `apps/web/src/components/theme-provider.tsx`
- Create: `apps/web/src/components/mode-toggle.tsx`
- Modify: `apps/web/src/main.tsx`

**Step 1: Create ThemeProvider**

```tsx
import { createContext, useContext, useEffect, useState } from 'react';

type Theme = 'dark' | 'light' | 'system';

interface ThemeProviderProps {
  children: React.ReactNode;
  defaultTheme?: Theme;
  storageKey?: string;
}

interface ThemeProviderState {
  theme: Theme;
  setTheme: (theme: Theme) => void;
}

const initialState: ThemeProviderState = {
  theme: 'system',
  setTheme: () => null,
};

const ThemeProviderContext = createContext<ThemeProviderState>(initialState);

export function ThemeProvider({
  children,
  defaultTheme = 'system',
  storageKey = 'gitcard-theme',
  ...props
}: ThemeProviderProps) {
  const [theme, setTheme] = useState<Theme>(
    () => (localStorage.getItem(storageKey) as Theme) || defaultTheme,
  );

  useEffect(() => {
    const root = window.document.documentElement;
    root.classList.remove('light', 'dark');

    if (theme === 'system') {
      const systemTheme = window.matchMedia('(prefers-color-scheme: dark)').matches
        ? 'dark'
        : 'light';
      root.classList.add(systemTheme);
      return;
    }

    root.classList.add(theme);
  }, [theme]);

  const value = {
    theme,
    setTheme: (theme: Theme) => {
      localStorage.setItem(storageKey, theme);
      setTheme(theme);
    },
  };

  return (
    <ThemeProviderContext.Provider {...props} value={value}>
      {children}
    </ThemeProviderContext.Provider>
  );
}

export const useTheme = (): ThemeProviderState => {
  const context = useContext(ThemeProviderContext);
  if (context === undefined) throw new Error('useTheme must be used within a ThemeProvider');
  return context;
};
```

**Step 2: Create ModeToggle**

```tsx
import { Moon, Sun } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { useTheme } from '@/components/theme-provider';

export function ModeToggle() {
  const { setTheme } = useTheme();

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="icon">
          <Sun className="h-[1.2rem] w-[1.2rem] scale-100 rotate-0 transition-all dark:scale-0 dark:-rotate-90" />
          <Moon className="absolute h-[1.2rem] w-[1.2rem] scale-0 rotate-90 transition-all dark:scale-100 dark:rotate-0" />
          <span className="sr-only">Toggle theme</span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem onClick={() => setTheme('light')}>Light</DropdownMenuItem>
        <DropdownMenuItem onClick={() => setTheme('dark')}>Dark</DropdownMenuItem>
        <DropdownMenuItem onClick={() => setTheme('system')}>System</DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
```

**Step 3: Wrap App in ThemeProvider in `main.tsx`**

```tsx
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { ThemeProvider } from './components/theme-provider';
import { App } from './App';
import './index.css';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ThemeProvider defaultTheme="system">
      <App />
    </ThemeProvider>
  </StrictMode>,
);
```

**Step 4: Verify typecheck + build**

```bash
pnpm --filter web typecheck && pnpm --filter web build
```

**Step 5: Commit**

```bash
git add apps/web/src/
git commit -m "feat(web): add ThemeProvider and ModeToggle for dark mode"
```

---

## Task 5: Rewrite Sidebar with tabbed layout

**Files:**

- Rewrite: `apps/web/src/components/Sidebar.tsx`
- Delete: `apps/web/src/components/OptionToggles.tsx`
- Delete: `apps/web/src/components/ThemePicker.tsx`
- Rewrite: `apps/web/src/components/ColorPicker.tsx`
- Rewrite: `apps/web/src/components/EmbedOutput.tsx`

**Step 1: Rewrite Sidebar.tsx with 3 tabs**

The sidebar uses shadcn `Tabs`, `Input`, `Checkbox`, `Switch`, `Select`, `Label`, `Separator`. It has 3 tabs: Configure, Colors, Export. The OptionToggles and ThemePicker components are inlined.

```tsx
import { THEME_NAMES } from '@gitcard/svg-renderer';
import type { CardConfig, CardType } from '@/hooks/useCardConfig';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Checkbox } from '@/components/ui/checkbox';
import { Switch } from '@/components/ui/switch';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Separator } from '@/components/ui/separator';
import { ColorPicker } from './ColorPicker';
import { EmbedOutput } from './EmbedOutput';

interface SidebarProps {
  config: CardConfig;
  cards: CardType[];
  onUsernameChange: (username: string) => void;
  onToggleCard: (card: CardType) => void;
  onThemeChange: (theme: string) => void;
  onToggle: (key: string, value: boolean) => void;
  onColorChange: (key: string, value: string) => void;
  buildSrc: (card: CardType) => string;
  hasData: boolean;
}

const CARD_LABELS: Record<CardType, string> = {
  stats: 'Stats',
  streak: 'Streak',
  'top-langs': 'Top Languages',
};

const ALL_CARDS: CardType[] = ['stats', 'streak', 'top-langs'];

const COLOR_FIELDS = [
  { key: 'bgColor', label: 'Background' },
  { key: 'titleColor', label: 'Title' },
  { key: 'textColor', label: 'Text' },
  { key: 'iconColor', label: 'Icon' },
  { key: 'borderColor', label: 'Border' },
] as const;

export function Sidebar({
  config,
  cards,
  onUsernameChange,
  onToggleCard,
  onThemeChange,
  onToggle,
  onColorChange,
  buildSrc,
  hasData,
}: SidebarProps) {
  return (
    <aside className="sticky top-[65px] self-start max-h-[calc(100vh-65px-3rem)] overflow-y-auto rounded-lg border bg-card p-4 shadow-sm">
      <Tabs defaultValue="configure">
        <TabsList className="grid w-full grid-cols-3">
          <TabsTrigger value="configure">Configure</TabsTrigger>
          <TabsTrigger value="colors">Colors</TabsTrigger>
          <TabsTrigger value="export">Export</TabsTrigger>
        </TabsList>

        <TabsContent value="configure" className="flex flex-col gap-4 pt-2">
          {/* Username */}
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="username-input">GitHub Username</Label>
            <Input
              id="username-input"
              value={config.username}
              onChange={(e) => onUsernameChange(e.target.value)}
              placeholder="e.g. octocat"
              autoComplete="off"
              spellCheck={false}
            />
          </div>

          <Separator />

          {/* Card selection */}
          <div className="flex flex-col gap-1.5">
            <Label>Cards</Label>
            <div className="flex flex-col gap-2">
              {ALL_CARDS.map((card) => (
                <label key={card} className="flex items-center gap-2 text-sm cursor-pointer">
                  <Checkbox
                    checked={config.cards.includes(card)}
                    onCheckedChange={() => onToggleCard(card)}
                  />
                  {CARD_LABELS[card]}
                </label>
              ))}
            </div>
          </div>

          <Separator />

          {/* Theme */}
          <div className="flex flex-col gap-1.5">
            <Label>Theme</Label>
            <Select value={config.theme} onValueChange={onThemeChange}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {THEME_NAMES.map((theme) => (
                  <SelectItem key={theme} value={theme}>
                    {theme.charAt(0).toUpperCase() + theme.slice(1)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <Separator />

          {/* Options */}
          <div className="flex flex-col gap-3">
            <Label>Options</Label>
            <div className="flex items-center justify-between">
              <Label htmlFor="show-icons" className="text-sm font-normal">
                Show icons
              </Label>
              <Switch
                id="show-icons"
                checked={config.showIcons}
                onCheckedChange={(v) => onToggle('showIcons', v)}
              />
            </div>
            <div className="flex items-center justify-between">
              <Label htmlFor="hide-border" className="text-sm font-normal">
                Hide border
              </Label>
              <Switch
                id="hide-border"
                checked={config.hideBorder}
                onCheckedChange={(v) => onToggle('hideBorder', v)}
              />
            </div>
            <div className="flex items-center justify-between">
              <Label htmlFor="hide-title" className="text-sm font-normal">
                Hide title
              </Label>
              <Switch
                id="hide-title"
                checked={config.hideTitle}
                onCheckedChange={(v) => onToggle('hideTitle', v)}
              />
            </div>
          </div>
        </TabsContent>

        <TabsContent value="colors" className="flex flex-col gap-3 pt-2">
          {COLOR_FIELDS.map(({ key, label }) => (
            <ColorPicker
              key={key}
              label={label}
              value={config[key as keyof CardConfig] as string}
              onChange={(v) => onColorChange(key, v)}
            />
          ))}
        </TabsContent>

        <TabsContent value="export" className="pt-2">
          {hasData ? (
            <EmbedOutput cards={cards} buildSrc={buildSrc} />
          ) : (
            <p className="text-sm text-muted-foreground">
              Enter a valid username to see embed codes.
            </p>
          )}
        </TabsContent>
      </Tabs>
    </aside>
  );
}
```

**Step 2: Rewrite ColorPicker.tsx with shadcn Input + Button**

```tsx
import { X } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';

interface ColorPickerProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
}

export function ColorPicker({ label, value, onChange }: ColorPickerProps) {
  const id = `color-${label.toLowerCase()}`;

  return (
    <div className="flex flex-col gap-1.5">
      <Label htmlFor={id}>{label}</Label>
      <div className="flex items-center gap-2">
        <input
          type="color"
          value={value ? `#${value}` : '#000000'}
          onChange={(e) => onChange(e.target.value.slice(1))}
          className="h-8 w-8 shrink-0 cursor-pointer rounded border border-input bg-transparent p-0.5"
        />
        <Input
          id={id}
          value={value}
          onChange={(e) => {
            const clean = e.target.value.replace(/[^0-9a-fA-F]/g, '').slice(0, 6);
            onChange(clean);
          }}
          placeholder="hex"
          className="font-mono text-xs"
          maxLength={6}
        />
        {value && (
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7 shrink-0"
            onClick={() => onChange('')}
          >
            <X className="h-3.5 w-3.5" />
          </Button>
        )}
      </div>
    </div>
  );
}
```

**Step 3: Rewrite EmbedOutput.tsx with shadcn Button + Badge**

```tsx
import { useState, useCallback } from 'react';
import { Copy, Check } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import type { CardType } from '@/hooks/useCardConfig';

interface EmbedOutputProps {
  cards: CardType[];
  buildSrc: (card: CardType) => string;
}

const CARD_ALT: Record<CardType, string> = {
  stats: 'GitHub Stats',
  streak: 'GitHub Streak',
  'top-langs': 'Top Languages',
};

const API_BASE = import.meta.env.VITE_API_URL ?? '';

export function EmbedOutput({ cards, buildSrc }: EmbedOutputProps) {
  const [copiedType, setCopiedType] = useState<string | null>(null);

  const markdownLines = cards.map((card) => {
    const src = `${API_BASE}${buildSrc(card)}`;
    return `![${CARD_ALT[card]}](${src})`;
  });

  const htmlLines = cards.map((card) => {
    const src = `${API_BASE}${buildSrc(card)}`;
    return `<img src="${src}" alt="${CARD_ALT[card]}" />`;
  });

  const markdown = markdownLines.join('\n');
  const html = htmlLines.join('\n');

  const handleCopy = useCallback(async (text: string, type: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopiedType(type);
      setTimeout(() => setCopiedType(null), 2000);
    } catch {
      // clipboard API may not be available
    }
  }, []);

  return (
    <div className="flex flex-col gap-4">
      <div>
        <div className="flex items-center justify-between mb-2">
          <span className="text-sm font-medium">Markdown</span>
          <Button variant="ghost" size="sm" onClick={() => handleCopy(markdown, 'markdown')}>
            {copiedType === 'markdown' ? (
              <>
                <Check className="mr-1 h-3 w-3" />
                Copied
              </>
            ) : (
              <>
                <Copy className="mr-1 h-3 w-3" />
                Copy
              </>
            )}
          </Button>
        </div>
        <pre className="rounded-md bg-muted p-3 text-xs font-mono overflow-x-auto">
          <code>{markdown}</code>
        </pre>
      </div>

      <Separator />

      <div>
        <div className="flex items-center justify-between mb-2">
          <span className="text-sm font-medium">HTML</span>
          <Button variant="ghost" size="sm" onClick={() => handleCopy(html, 'html')}>
            {copiedType === 'html' ? (
              <>
                <Check className="mr-1 h-3 w-3" />
                Copied
              </>
            ) : (
              <>
                <Copy className="mr-1 h-3 w-3" />
                Copy
              </>
            )}
          </Button>
        </div>
        <pre className="rounded-md bg-muted p-3 text-xs font-mono overflow-x-auto">
          <code>{html}</code>
        </pre>
      </div>
    </div>
  );
}
```

**Step 4: Delete `OptionToggles.tsx` and `ThemePicker.tsx`**

These are now inlined in Sidebar.tsx.

**Step 5: Verify typecheck**

```bash
pnpm --filter web typecheck
```

**Step 6: Commit**

```bash
git add apps/web/src/
git commit -m "feat(web): rewrite Sidebar with shadcn Tabs, inline options and theme picker"
```

---

## Task 6: Rewrite CardPreview and CardList with Tailwind

**Files:**

- Rewrite: `apps/web/src/components/CardPreview.tsx`
- Rewrite: `apps/web/src/components/CardList.tsx`

**Step 1: Rewrite CardPreview.tsx**

Same render logic, Tailwind classes instead of CSS classes:

```tsx
import { useMemo } from 'react';
import {
  renderStatsCard,
  renderStreakCard,
  renderLangsCard,
  type CardOptions,
} from '@gitcard/svg-renderer';
import type { CardType } from '@/hooks/useCardConfig';
import type { GitHubData } from '@/hooks/useGitHubData';

interface CardPreviewProps {
  id: CardType;
  data: GitHubData;
  options: CardOptions;
}

const renderers: Record<CardType, (data: GitHubData, options: CardOptions) => string> = {
  stats: (data, options) => renderStatsCard(data.stats, options),
  streak: (data, options) => renderStreakCard(data.streak, options),
  'top-langs': (data, options) => renderLangsCard(data.languages, options),
};

export function CardPreview({ id, data, options }: CardPreviewProps) {
  const svg = useMemo(() => renderers[id](data, options), [id, data, options]);
  return <div className="w-full max-w-[495px] p-4" dangerouslySetInnerHTML={{ __html: svg }} />;
}
```

**Step 2: Rewrite CardList.tsx with Tailwind + shadcn Card**

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
import { GripVertical } from 'lucide-react';
import type { CardOptions } from '@gitcard/svg-renderer';
import type { CardType } from '@/hooks/useCardConfig';
import type { GitHubData } from '@/hooks/useGitHubData';
import { CardPreview } from './CardPreview';

interface CardListProps {
  cards: CardType[];
  onReorder: (cards: CardType[]) => void;
  data: GitHubData;
  options: CardOptions;
}

interface SortableCardProps {
  id: CardType;
  data: GitHubData;
  options: CardOptions;
}

function SortableCard({ id, data, options }: SortableCardProps) {
  const { attributes, listeners, setNodeRef, transform, transition } = useSortable({ id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className="rounded-lg border bg-card shadow-sm cursor-grab overflow-hidden transition-shadow hover:shadow-md active:cursor-grabbing active:shadow-lg"
      {...attributes}
      {...listeners}
    >
      <div className="flex items-center gap-2 px-3 py-2 border-b bg-muted/50">
        <GripVertical className="h-4 w-4 text-muted-foreground" />
      </div>
      <CardPreview id={id} data={data} options={options} />
    </div>
  );
}

export function CardList({ cards, onReorder, data, options }: CardListProps) {
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
        <div className="flex flex-col gap-4">
          {cards.map((card) => (
            <SortableCard key={card} id={card} data={data} options={options} />
          ))}
        </div>
      </SortableContext>
    </DndContext>
  );
}
```

**Step 3: Commit**

```bash
git add apps/web/src/components/
git commit -m "feat(web): rewrite CardPreview and CardList with Tailwind and shadcn Card"
```

---

## Task 7: Rewrite App.tsx with Tailwind layout + ModeToggle

**Files:**

- Rewrite: `apps/web/src/App.tsx`

**Step 1: Rewrite App.tsx**

Update layout to Tailwind grid, add ModeToggle in header, pass new props to Sidebar (including buildSrc and hasData for the Export tab), remove old CSS import:

```tsx
import { useState, useEffect, useRef, useCallback } from 'react';
import { useCardConfig } from '@/hooks/useCardConfig';
import { useGitHubData } from '@/hooks/useGitHubData';
import type { CardConfig, CardType } from '@/hooks/useCardConfig';
import { Sidebar } from '@/components/Sidebar';
import { CardList } from '@/components/CardList';
import { ModeToggle } from '@/components/mode-toggle';

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
    cardOptions,
  } = useCardConfig();

  const [debouncedUsername, setDebouncedUsername] = useState('');
  const debounceRef = useRef<ReturnType<typeof setTimeout>>(null);

  useEffect(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }

    if (!config.username.trim()) {
      setDebouncedUsername('');
      return;
    }

    debounceRef.current = setTimeout(() => {
      setDebouncedUsername(config.username.trim());
    }, 300);

    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, [config.username]);

  const { data, isLoading, error } = useGitHubData(debouncedUsername);

  const buildSrc = useCallback(
    (card: CardType): string => {
      return `${cardPaths[card]}${buildQueryString(card)}`;
    },
    [cardPaths, buildQueryString],
  );

  const handleToggle = useCallback(
    (key: string, value: boolean) => {
      setOption(key as keyof CardConfig, value as CardConfig[keyof CardConfig]);
    },
    [setOption],
  );

  const handleColorChange = useCallback(
    (key: string, value: string) => {
      setOption(key as keyof CardConfig, value as CardConfig[keyof CardConfig]);
    },
    [setOption],
  );

  const showPreview = data !== null;

  return (
    <div className="flex min-h-screen flex-col">
      {/* Header */}
      <header className="sticky top-0 z-50 flex items-center justify-between border-b bg-background/95 px-6 py-3 shadow-sm backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="flex items-baseline gap-3">
          <h1 className="text-xl font-bold tracking-tight text-primary">GitCard</h1>
          <p className="hidden text-sm text-muted-foreground sm:block">
            GitHub Stats Card Configurator
          </p>
        </div>
        <ModeToggle />
      </header>

      {/* Body */}
      <div className="mx-auto grid w-full max-w-[1200px] flex-1 grid-cols-1 gap-6 p-6 md:grid-cols-[300px_1fr]">
        <Sidebar
          config={config}
          cards={config.cards}
          onUsernameChange={setUsername}
          onToggleCard={toggleCard}
          onThemeChange={setTheme}
          onToggle={handleToggle}
          onColorChange={handleColorChange}
          buildSrc={buildSrc}
          hasData={showPreview}
        />

        <main className="flex min-w-0 flex-col gap-6">
          {!config.username.trim() && (
            <div className="flex min-h-[300px] items-center justify-center rounded-lg border-2 border-dashed">
              <p className="text-muted-foreground">Enter a GitHub username to get started</p>
            </div>
          )}

          {config.username.trim() && isLoading && (
            <div className="flex min-h-[300px] items-center justify-center rounded-lg border-2 border-dashed">
              <div className="flex flex-col items-center gap-2">
                <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent" />
                <p className="text-sm text-muted-foreground">Loading...</p>
              </div>
            </div>
          )}

          {config.username.trim() && error && !isLoading && (
            <div className="flex min-h-[300px] items-center justify-center rounded-lg border-2 border-dashed border-destructive bg-destructive/5">
              <p className="text-destructive">{error}</p>
            </div>
          )}

          {showPreview && (
            <CardList cards={config.cards} onReorder={setCards} data={data} options={cardOptions} />
          )}
        </main>
      </div>
    </div>
  );
}
```

**Step 2: Verify full build + typecheck**

```bash
pnpm --filter web typecheck && pnpm --filter web build
```

**Step 3: Commit**

```bash
git add apps/web/src/
git commit -m "feat(web): rewrite App.tsx with Tailwind layout and ModeToggle"
```

---

## Task 8: Clean up and verify

**Files:**

- Delete: `apps/web/src/styles/app.css` (if not already deleted)
- Delete: `apps/web/src/components/OptionToggles.tsx`
- Delete: `apps/web/src/components/ThemePicker.tsx`

**Step 1: Delete old files**

```bash
rm -f apps/web/src/styles/app.css apps/web/src/components/OptionToggles.tsx apps/web/src/components/ThemePicker.tsx
rmdir apps/web/src/styles 2>/dev/null || true
```

**Step 2: Run full check suite**

```bash
pnpm install && pnpm check && pnpm build && pnpm test
```

All must pass.

**Step 3: Visual test**

```bash
pnpm dev
```

Open `http://localhost:5173`:

- Verify light/dark/system theme toggle works
- Enter a username, verify card previews render
- Switch tabs (Configure, Colors, Export)
- Change theme/colors — verify instant re-render
- Test drag-and-drop card reordering
- Test responsive layout at < 768px

**Step 4: Commit**

```bash
git add -A
git commit -m "feat(web): complete shadcn/ui rewrite with dark mode and tabbed sidebar"
```
