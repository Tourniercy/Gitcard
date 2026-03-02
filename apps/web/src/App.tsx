import { GitHubLogoIcon } from '@radix-ui/react-icons';
import { CardConfigProvider } from '@/components/CardConfigProvider';
import { useCardConfigContext } from '@/components/CardConfigProvider';
import { CardPreviewArea } from '@/components/CardPreviewArea';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Button } from '@/components/ui/button';
import { MultiSelect } from '@/components/ui/multi-select';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { ThemeCombobox } from '@/components/ThemeCombobox';
import { LocaleCombobox } from '@/components/LocaleCombobox';
import { ColorInput } from '@/components/ColorInput';
import { ModeToggle } from '@/components/mode-toggle';
import { Sliders, Palette } from 'lucide-react';

function MainContent() {
  const { config, setUsername, setTheme, setOption, handleToggle } = useCardConfigContext();

  return (
    <div className="container flex flex-col gap-6 py-6">
      {/* Floating toolbar */}
      <div className="flex flex-wrap items-center gap-2 rounded-lg border bg-card p-3 shadow-sm">
        <Input
          value={config.username}
          onChange={(e) => setUsername(e.target.value)}
          placeholder="GitHub username"
          className="min-w-[180px] flex-1"
          autoComplete="off"
          spellCheck={false}
        />
        <div className="w-48">
          <ThemeCombobox value={config.theme} onValueChange={setTheme} />
        </div>
        <div className="w-48">
          <LocaleCombobox value={config.locale} onValueChange={(v) => setOption('locale', v)} />
        </div>

        {/* Options popover */}
        <Popover>
          <PopoverTrigger asChild>
            <Button variant="outline" className="gap-1.5">
              <Sliders className="h-3.5 w-3.5" />
              Options
            </Button>
          </PopoverTrigger>
          <PopoverContent className="w-48" align="start">
            <div className="flex flex-col gap-3">
              <div className="flex items-center justify-between">
                <Label htmlFor="show-icons" className="text-sm font-normal">
                  Show icons
                </Label>
                <Switch
                  id="show-icons"
                  checked={config.showIcons}
                  onCheckedChange={(v) => handleToggle('showIcons', v)}
                />
              </div>
              <div className="flex items-center justify-between">
                <Label htmlFor="hide-border" className="text-sm font-normal">
                  Hide border
                </Label>
                <Switch
                  id="hide-border"
                  checked={config.hideBorder}
                  onCheckedChange={(v) => handleToggle('hideBorder', v)}
                />
              </div>
              <div className="flex items-center justify-between">
                <Label htmlFor="hide-title" className="text-sm font-normal">
                  Hide title
                </Label>
                <Switch
                  id="hide-title"
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
            <Button variant="outline" className="gap-1.5">
              <Palette className="h-3.5 w-3.5" />
              Colors
            </Button>
          </PopoverTrigger>
          <PopoverContent className="w-48" align="start">
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
                  id={`color-${key}`}
                  label={label}
                  value={config[key]}
                  onChange={(v) => setOption(key, v)}
                />
              ))}
            </div>
          </PopoverContent>
        </Popover>

        {/* Hide stats multi-select */}
        <div className="w-90">
          <MultiSelect
            options={[
              { label: 'Stars', value: 'stars' },
              { label: 'Commits', value: 'commits' },
              { label: 'PRs', value: 'prs' },
              { label: 'Issues', value: 'issues' },
              { label: 'Forks', value: 'forks' },
              { label: 'Contribs', value: 'contribs' },
            ]}
            defaultValue={config.hide}
            onValueChange={(values) => setOption('hide', values)}
            placeholder="Hide stats..."
            maxCount={2}
            hideSelectAll
            searchable={false}
          />
        </div>
      </div>

      {/* Cards */}
      <CardPreviewArea gridClassName="grid grid-cols-1 gap-4 lg:grid-cols-2" />
    </div>
  );
}

export function App() {
  return (
    <div className="flex min-h-screen flex-col">
      <header className="sticky top-0 z-50 w-full border-b border-border/40 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="container flex h-14 items-center">
          <a href="/" className="mr-2 flex items-center md:mr-6 md:space-x-2">
            <span className="font-bold">GitCard</span>
          </a>
          <nav className="flex flex-1 items-center justify-end gap-1">
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
          </nav>
        </div>
      </header>
      <CardConfigProvider>
        <MainContent />
      </CardConfigProvider>
    </div>
  );
}
