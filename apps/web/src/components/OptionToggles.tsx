interface OptionTogglesProps {
  showIcons: boolean;
  hideBorder: boolean;
  hideTitle: boolean;
  onToggle: (key: string, value: boolean) => void;
}

export function OptionToggles({ showIcons, hideBorder, hideTitle, onToggle }: OptionTogglesProps) {
  return (
    <div className="control-group">
      <label>Options</label>
      <div className="toggle-list">
        <label className="toggle-item">
          <input
            type="checkbox"
            checked={showIcons}
            onChange={(e) => onToggle('showIcons', e.target.checked)}
          />
          <span>Show icons</span>
        </label>
        <label className="toggle-item">
          <input
            type="checkbox"
            checked={hideBorder}
            onChange={(e) => onToggle('hideBorder', e.target.checked)}
          />
          <span>Hide border</span>
        </label>
        <label className="toggle-item">
          <input
            type="checkbox"
            checked={hideTitle}
            onChange={(e) => onToggle('hideTitle', e.target.checked)}
          />
          <span>Hide title</span>
        </label>
      </div>
    </div>
  );
}
