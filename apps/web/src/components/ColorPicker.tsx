interface ColorPickerProps {
  label: string;
  value: string;
  onChange: (hex: string) => void;
}

export function ColorPicker({ label, value, onChange }: ColorPickerProps) {
  const colorValue = value ? `#${value}` : '#000000';
  const inputId = `color-${label.toLowerCase().replace(/\s+/g, '-')}`;

  return (
    <div className="control-group color-control">
      <label htmlFor={inputId}>{label}</label>
      <div className="color-input-row">
        <input
          type="color"
          id={inputId}
          value={colorValue}
          onChange={(e) => onChange(e.target.value.replace('#', ''))}
        />
        <input
          type="text"
          className="color-hex-input"
          value={value}
          onChange={(e) => {
            const hex = e.target.value.replace(/[^0-9a-fA-F]/g, '').slice(0, 6);
            onChange(hex);
          }}
          placeholder="hex color"
          maxLength={6}
        />
        {value && (
          <button
            type="button"
            className="color-clear-btn"
            onClick={() => onChange('')}
            aria-label={`Clear ${label}`}
          >
            x
          </button>
        )}
      </div>
    </div>
  );
}
