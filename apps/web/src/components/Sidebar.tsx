import type { CardConfig, CardType } from '@/hooks/useCardConfig';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Checkbox } from '@/components/ui/checkbox';
import { Switch } from '@/components/ui/switch';
import { Separator } from '@/components/ui/separator';
import { ThemeCombobox } from './ThemeCombobox';
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
