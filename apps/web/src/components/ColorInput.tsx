import { useState } from 'react';
import { HexColorPicker } from 'react-colorful';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';

interface ColorInputProps {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
}

export function ColorInput({ id, label, value, onChange }: ColorInputProps) {
  const [open, setOpen] = useState(false);
  const hex = value ? `#${value}` : '';

  return (
    <div className="flex flex-col gap-1">
      <Label htmlFor={id} className="text-xs font-normal text-muted-foreground">
        {label}
      </Label>
      <div className="flex items-center gap-2">
        <Popover open={open} onOpenChange={setOpen}>
          <PopoverTrigger asChild>
            <button
              type="button"
              className="h-8 w-8 shrink-0 rounded-md border border-border shadow-sm"
              style={{ backgroundColor: hex || 'transparent' }}
              aria-label={`Pick ${label.toLowerCase()} color`}
            />
          </PopoverTrigger>
          <PopoverContent className="w-auto p-3" align="start">
            <HexColorPicker
              color={hex || '#000000'}
              onChange={(c) => onChange(c.replace('#', ''))}
            />
          </PopoverContent>
        </Popover>
        <Input
          id={id}
          value={value}
          onChange={(e) => onChange(e.target.value.replace('#', ''))}
          placeholder="0d1117"
          className="h-8 flex-1 text-xs font-mono"
        />
      </div>
    </div>
  );
}
