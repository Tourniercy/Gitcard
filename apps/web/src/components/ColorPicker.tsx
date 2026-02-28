import { X } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';

interface ColorPickerProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
}

export function ColorPicker({ label, value, onChange }: ColorPickerProps) {
  const id = `color-${label.toLowerCase()}`;

  return (
    <div className="flex flex-col gap-1.5">
      <Label htmlFor={id}>{label}</Label>
      <div className="flex items-center gap-2">
        <input
          type="color"
          value={value ? `#${value}` : '#000000'}
          onChange={(e) => onChange(e.target.value.slice(1))}
          className="h-8 w-8 shrink-0 cursor-pointer rounded border border-input bg-transparent p-0.5"
        />
        <Input
          id={id}
          value={value}
          onChange={(e) => {
            const clean = e.target.value.replace(/[^0-9a-fA-F]/g, '').slice(0, 6);
            onChange(clean);
          }}
          placeholder="hex"
          className="font-mono text-xs"
          maxLength={6}
        />
        {value && (
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7 shrink-0"
            onClick={() => onChange('')}
          >
            <X className="h-3.5 w-3.5" />
          </Button>
        )}
      </div>
    </div>
  );
}
