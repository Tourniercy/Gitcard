import type { CardConfig, CardType } from '../hooks/useCardConfig';
import { ThemePicker } from './ThemePicker';
import { OptionToggles } from './OptionToggles';
import { ColorPicker } from './ColorPicker';

interface SidebarProps {
  config: CardConfig;
  onUsernameChange: (username: string) => void;
  onToggleCard: (card: CardType) => void;
  onThemeChange: (theme: string) => void;
  onToggle: (key: string, value: boolean) => void;
  onColorChange: (key: string, value: string) => void;
}

const CARD_LABELS: Record<CardType, string> = {
  stats: 'Stats',
  streak: 'Streak',
  'top-langs': 'Top Languages',
};

const ALL_CARDS: CardType[] = ['stats', 'streak', 'top-langs'];

export function Sidebar({
  config,
  onUsernameChange,
  onToggleCard,
  onThemeChange,
  onToggle,
  onColorChange,
}: SidebarProps) {
  return (
    <aside className="sidebar">
      <div className="control-group">
        <label htmlFor="username-input">GitHub Username</label>
        <input
          id="username-input"
          type="text"
          value={config.username}
          onChange={(e) => onUsernameChange(e.target.value)}
          placeholder="e.g. octocat"
          autoComplete="off"
          spellCheck={false}
        />
      </div>

      <div className="control-group">
        <label>Cards</label>
        <div className="toggle-list">
          {ALL_CARDS.map((card) => (
            <label key={card} className="toggle-item">
              <input
                type="checkbox"
                checked={config.cards.includes(card)}
                onChange={() => onToggleCard(card)}
              />
              <span>{CARD_LABELS[card]}</span>
            </label>
          ))}
        </div>
      </div>

      <ThemePicker value={config.theme} onChange={onThemeChange} />

      <OptionToggles
        showIcons={config.showIcons}
        hideBorder={config.hideBorder}
        hideTitle={config.hideTitle}
        onToggle={onToggle}
      />

      <div className="control-group">
        <label className="section-label">Colors</label>
        <ColorPicker
          label="Background"
          value={config.bgColor}
          onChange={(v) => onColorChange('bgColor', v)}
        />
        <ColorPicker
          label="Title"
          value={config.titleColor}
          onChange={(v) => onColorChange('titleColor', v)}
        />
        <ColorPicker
          label="Text"
          value={config.textColor}
          onChange={(v) => onColorChange('textColor', v)}
        />
        <ColorPicker
          label="Icon"
          value={config.iconColor}
          onChange={(v) => onColorChange('iconColor', v)}
        />
        <ColorPicker
          label="Border"
          value={config.borderColor}
          onChange={(v) => onColorChange('borderColor', v)}
        />
      </div>
    </aside>
  );
}
