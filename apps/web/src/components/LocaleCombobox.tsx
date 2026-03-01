import { useState } from 'react';
import { Check, ChevronsUpDown } from 'lucide-react';
import { SUPPORTED_LOCALES } from '@gitcard/svg-renderer';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import {
  Command,
  CommandEmpty,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/components/ui/command';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';

interface LocaleComboboxProps {
  value: string;
  onValueChange: (value: string) => void;
}

function getLocale(code: string) {
  return SUPPORTED_LOCALES.find((l) => l.code === code) ?? SUPPORTED_LOCALES[0];
}

export function LocaleCombobox({ value, onValueChange }: LocaleComboboxProps) {
  const [open, setOpen] = useState(false);
  const current = getLocale(value);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          role="combobox"
          aria-expanded={open}
          className="w-full justify-between font-normal"
          size="sm"
        >
          <div className="flex items-center gap-2 min-w-0">
            <span className="shrink-0">{current.flag}</span>
            <span className="truncate">{current.label}</span>
          </div>
          <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-[--radix-popover-trigger-width] p-0" align="start">
        <Command defaultValue={value}>
          <CommandInput placeholder="Search locales..." />
          <CommandList className="max-h-[250px]">
            <CommandEmpty>No locale found.</CommandEmpty>
            {SUPPORTED_LOCALES.map((l) => (
              <CommandItem
                key={l.code}
                value={l.code}
                keywords={[l.label]}
                onSelect={(v: string) => {
                  onValueChange(v);
                  setOpen(false);
                }}
              >
                <span className="shrink-0">{l.flag}</span>
                <span>{l.label}</span>
                <Check
                  className={cn('ml-auto h-4 w-4', value === l.code ? 'opacity-100' : 'opacity-0')}
                />
              </CommandItem>
            ))}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}
