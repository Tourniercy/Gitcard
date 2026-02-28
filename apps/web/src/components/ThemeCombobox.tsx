import { useState } from 'react';
import { Check, ChevronsUpDown } from 'lucide-react';
import { getTheme, THEME_GROUPS } from '@gitcard/svg-renderer';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/components/ui/command';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';

interface ThemeComboboxProps {
  value: string;
  onValueChange: (value: string) => void;
}

function ThemeSwatch({ themeName }: { themeName: string }) {
  const theme = getTheme(themeName);
  return (
    <div className="flex gap-0.5 shrink-0" aria-hidden>
      <span
        className="h-4 w-2 rounded-l-sm border border-border/50"
        style={{ backgroundColor: theme.title }}
      />
      <span className="h-4 w-2 border-y border-border/50" style={{ backgroundColor: theme.icon }} />
      <span className="h-4 w-2 border-y border-border/50" style={{ backgroundColor: theme.text }} />
      <span
        className="h-4 w-2 rounded-r-sm border border-border/50"
        style={{ backgroundColor: theme.muted }}
      />
    </div>
  );
}

function formatName(name: string): string {
  return name
    .split('-')
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(' ');
}

export function ThemeCombobox({ value, onValueChange }: ThemeComboboxProps) {
  const [open, setOpen] = useState(false);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          role="combobox"
          aria-expanded={open}
          className="w-full justify-between font-normal"
        >
          <div className="flex items-center gap-2 min-w-0">
            <ThemeSwatch themeName={value} />
            <span className="truncate">{formatName(value)}</span>
          </div>
          <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-[--radix-popover-trigger-width] p-0" align="start">
        <Command>
          <CommandInput placeholder="Search themes..." />
          <CommandList className="max-h-[250px]">
            <CommandEmpty>No theme found.</CommandEmpty>
            {THEME_GROUPS.map((group) => (
              <CommandGroup key={group.label} heading={group.label}>
                {group.themes.map((theme) => (
                  <CommandItem
                    key={theme}
                    value={theme}
                    onSelect={(v: string) => {
                      onValueChange(v);
                      setOpen(false);
                    }}
                  >
                    <ThemeSwatch themeName={theme} />
                    <span>{formatName(theme)}</span>
                    <Check
                      className={cn(
                        'ml-auto h-4 w-4',
                        value === theme ? 'opacity-100' : 'opacity-0',
                      )}
                    />
                  </CommandItem>
                ))}
              </CommandGroup>
            ))}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}
