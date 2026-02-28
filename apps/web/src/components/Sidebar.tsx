import type { CardConfig } from '@/hooks/useCardConfig';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Separator } from '@/components/ui/separator';
import { ThemeCombobox } from './ThemeCombobox';

interface SidebarProps {
  config: CardConfig;
  onUsernameChange: (username: string) => void;
  onThemeChange: (theme: string) => void;
  onToggle: (key: string, value: boolean) => void;
  onOptionChange: <K extends keyof CardConfig>(key: K, value: CardConfig[K]) => void;
}

export function Sidebar({
  config,
  onUsernameChange,
  onThemeChange,
  onToggle,
  onOptionChange,
}: SidebarProps) {
  return (
    <aside className="sticky top-[73px] self-start max-h-[calc(100vh-73px-3rem)] overflow-y-auto rounded-lg border bg-card p-4 shadow-sm">
      <div className="flex flex-col gap-4">
        {/* Username */}
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="username-input">GitHub Username</Label>
          <Input
            id="username-input"
            value={config.username}
            onChange={(e) => onUsernameChange(e.target.value)}
            placeholder="e.g. tourniercy"
            autoComplete="off"
            spellCheck={false}
          />
        </div>

        <Separator />

        {/* Theme */}
        <div className="flex flex-col gap-1.5">
          <Label>Theme</Label>
          <ThemeCombobox value={config.theme} onValueChange={onThemeChange} />
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

        <Separator />

        {/* Hide Stats */}
        <div className="flex flex-col gap-3">
          <Label>Hide Stats</Label>
          {(['stars', 'commits', 'prs', 'issues', 'forks', 'contribs'] as const).map((stat) => (
            <div key={stat} className="flex items-center gap-2">
              <Checkbox
                id={`hide-${stat}`}
                checked={config.hide.includes(stat)}
                onCheckedChange={(checked) => {
                  const next = checked
                    ? [...config.hide, stat]
                    : config.hide.filter((s) => s !== stat);
                  onOptionChange('hide', next);
                }}
              />
              <Label htmlFor={`hide-${stat}`} className="text-sm font-normal">
                {stat}
              </Label>
            </div>
          ))}
        </div>

        <Separator />

        {/* Custom Colors */}
        <div className="flex flex-col gap-3">
          <Label>Custom Colors</Label>
          {(
            [
              ['bgColor', 'Background'],
              ['titleColor', 'Title'],
              ['textColor', 'Text'],
              ['iconColor', 'Icon'],
              ['borderColor', 'Border'],
            ] as const
          ).map(([key, label]) => (
            <div key={key} className="flex flex-col gap-1">
              <Label htmlFor={`color-${key}`} className="text-xs font-normal text-muted-foreground">
                {label}
              </Label>
              <Input
                id={`color-${key}`}
                value={config[key]}
                onChange={(e) => onOptionChange(key, e.target.value)}
                placeholder="e.g. 0d1117"
                className="h-8 text-xs font-mono"
              />
            </div>
          ))}
        </div>

        <Separator />

        {/* Locale */}
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="locale-input">Locale</Label>
          <Input
            id="locale-input"
            value={config.locale}
            onChange={(e) => onOptionChange('locale', e.target.value)}
            placeholder="en"
            className="h-8"
          />
        </div>
      </div>
    </aside>
  );
}
