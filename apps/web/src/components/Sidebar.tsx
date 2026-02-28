import type { CardConfig } from '@/hooks/useCardConfig';
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
}

export function Sidebar({ config, onUsernameChange, onThemeChange, onToggle }: SidebarProps) {
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
            placeholder="e.g. octocat"
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
      </div>
    </aside>
  );
}
