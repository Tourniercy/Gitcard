import { THEME_NAMES } from '@gitcard/svg-renderer';

interface ThemePickerProps {
  value: string;
  onChange: (theme: string) => void;
}

export function ThemePicker({ value, onChange }: ThemePickerProps) {
  return (
    <div className="control-group">
      <label htmlFor="theme-select">Theme</label>
      <select id="theme-select" value={value} onChange={(e) => onChange(e.target.value)}>
        {THEME_NAMES.map((theme) => (
          <option key={theme} value={theme}>
            {theme.charAt(0).toUpperCase() + theme.slice(1)}
          </option>
        ))}
      </select>
    </div>
  );
}
