import { useState, useEffect } from 'react';

interface ThemePickerProps {
  value: string;
  onChange: (theme: string) => void;
}

export function ThemePicker({ value, onChange }: ThemePickerProps) {
  const [themes, setThemes] = useState<string[]>([]);

  useEffect(() => {
    fetch('/api/themes')
      .then((res) => res.json())
      .then((data: unknown) => {
        if (Array.isArray(data)) {
          setThemes(data as string[]);
        }
      })
      .catch(() => {
        // Fallback themes if API is unavailable
        setThemes(['default', 'dark', 'dracula']);
      });
  }, []);

  return (
    <div className="control-group">
      <label htmlFor="theme-select">Theme</label>
      <select id="theme-select" value={value} onChange={(e) => onChange(e.target.value)}>
        {themes.map((theme) => (
          <option key={theme} value={theme}>
            {theme.charAt(0).toUpperCase() + theme.slice(1)}
          </option>
        ))}
      </select>
    </div>
  );
}
