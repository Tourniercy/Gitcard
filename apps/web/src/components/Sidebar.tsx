import type { CardConfig } from '@/hooks/useCardConfig';
import { SUPPORTED_LOCALES } from '@gitcard/svg-renderer';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Separator } from '@/components/ui/separator';
import { ThemeCombobox } from './ThemeCombobox';
import { ColorInput } from './ColorInput';

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
            <ColorInput
              key={key}
              id={`color-${key}`}
              label={label}
              value={config[key]}
              onChange={(v) => onOptionChange(key, v)}
            />
          ))}
        </div>

        <Separator />

        {/* Locale */}
        <div className="flex flex-col gap-1.5">
          <Label>Locale</Label>
          <Select value={config.locale} onValueChange={(v) => onOptionChange('locale', v)}>
            <SelectTrigger size="sm" className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {SUPPORTED_LOCALES.map((l) => (
                <SelectItem key={l.code} value={l.code}>
                  {l.label} ({l.code})
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>
    </aside>
  );
}
