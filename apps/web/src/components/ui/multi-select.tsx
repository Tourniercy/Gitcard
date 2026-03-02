import * as React from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { CheckIcon, XCircle, ChevronDown, XIcon } from 'lucide-react';

import { cn } from '@/lib/utils';
import { Separator } from '@/components/ui/separator';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
} from '@/components/ui/command';

const multiSelectVariants = cva('m-1 transition-all duration-300 ease-in-out', {
  variants: {
    variant: {
      default: 'border-foreground/10 text-foreground bg-card hover:bg-card/80',
      secondary:
        'border-foreground/10 bg-secondary text-secondary-foreground hover:bg-secondary/80',
      destructive:
        'border-transparent bg-destructive text-destructive-foreground hover:bg-destructive/80',
      inverted: 'inverted',
    },
  },
  defaultVariants: {
    variant: 'default',
  },
});

interface MultiSelectOption {
  label: string;
  value: string;
  icon?: React.ComponentType<{ className?: string }>;
  disabled?: boolean;
}

interface MultiSelectProps
  extends
    Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, 'defaultValue'>,
    VariantProps<typeof multiSelectVariants> {
  options: MultiSelectOption[];
  onValueChange: (value: string[]) => void;
  defaultValue?: string[];
  placeholder?: string;
  maxCount?: number;
  modalPopover?: boolean;
  className?: string;
  hideSelectAll?: boolean;
  searchable?: boolean;
}

const MultiSelect = React.forwardRef<HTMLButtonElement, MultiSelectProps>(
  (
    {
      options,
      onValueChange,
      variant,
      defaultValue = [],
      placeholder = 'Select options',
      maxCount = 3,
      modalPopover = false,
      className,
      hideSelectAll = false,
      searchable = true,
      ...props
    },
    ref,
  ) => {
    const [selectedValues, setSelectedValues] = React.useState<string[]>(defaultValue);
    const [isPopoverOpen, setIsPopoverOpen] = React.useState(false);
    const [searchValue, setSearchValue] = React.useState('');

    const prevDefaultValueRef = React.useRef<string[]>(defaultValue);

    React.useEffect(() => {
      const prev = prevDefaultValueRef.current;
      if (prev.length !== defaultValue.length || prev.some((v, i) => v !== defaultValue[i])) {
        setSelectedValues(defaultValue);
        prevDefaultValueRef.current = defaultValue;
      }
    }, [defaultValue]);

    React.useEffect(() => {
      if (!isPopoverOpen) {
        setSearchValue('');
      }
    }, [isPopoverOpen]);

    const toggleOption = (optionValue: string) => {
      const newSelectedValues = selectedValues.includes(optionValue)
        ? selectedValues.filter((v) => v !== optionValue)
        : [...selectedValues, optionValue];
      setSelectedValues(newSelectedValues);
      onValueChange(newSelectedValues);
    };

    const handleClear = () => {
      setSelectedValues([]);
      onValueChange([]);
    };

    const toggleAll = () => {
      if (selectedValues.length === options.length) {
        handleClear();
      } else {
        const allValues = options.map((o) => o.value);
        setSelectedValues(allValues);
        onValueChange(allValues);
      }
    };

    const clearExtraOptions = () => {
      const newSelectedValues = selectedValues.slice(0, maxCount);
      setSelectedValues(newSelectedValues);
      onValueChange(newSelectedValues);
    };

    const filteredOptions = React.useMemo(() => {
      if (!searchable || !searchValue) return options;
      return options.filter(
        (option) =>
          option.label.toLowerCase().includes(searchValue.toLowerCase()) ||
          option.value.toLowerCase().includes(searchValue.toLowerCase()),
      );
    }, [options, searchValue, searchable]);

    return (
      <Popover open={isPopoverOpen} onOpenChange={setIsPopoverOpen} modal={modalPopover}>
        <PopoverTrigger asChild>
          <Button
            ref={ref}
            {...props}
            onClick={() => setIsPopoverOpen((prev) => !prev)}
            className={cn(
              'flex h-9 w-full items-center justify-between rounded-md border bg-inherit p-1 hover:bg-inherit [&_svg]:pointer-events-auto',
              className,
            )}
          >
            {selectedValues.length > 0 ? (
              <div className="flex w-full items-center justify-between">
                <div className="flex flex-wrap items-center gap-1">
                  {selectedValues.slice(0, maxCount).map((value) => {
                    const option = options.find((o) => o.value === value);
                    if (!option) return null;
                    const IconComponent = option.icon;
                    return (
                      <Badge
                        key={value}
                        className={cn(
                          multiSelectVariants({ variant }),
                          '[&>svg]:pointer-events-auto',
                        )}
                      >
                        {IconComponent && <IconComponent className="mr-1 h-3 w-3" />}
                        <span>{option.label}</span>
                        <div
                          role="button"
                          tabIndex={0}
                          onClick={(e) => {
                            e.stopPropagation();
                            toggleOption(value);
                          }}
                          onKeyDown={(e) => {
                            if (e.key === 'Enter' || e.key === ' ') {
                              e.preventDefault();
                              e.stopPropagation();
                              toggleOption(value);
                            }
                          }}
                          className="ml-1 cursor-pointer rounded-sm hover:bg-white/20"
                        >
                          <XCircle className="h-3 w-3" />
                        </div>
                      </Badge>
                    );
                  })}
                  {selectedValues.length > maxCount && (
                    <Badge
                      className={cn(
                        'bg-transparent text-foreground border-foreground/10 hover:bg-transparent',
                        multiSelectVariants({ variant }),
                        '[&>svg]:pointer-events-auto',
                      )}
                    >
                      {`+ ${selectedValues.length - maxCount} more`}
                      <XCircle
                        className="ml-1 h-3 w-3 cursor-pointer"
                        onClick={(e) => {
                          e.stopPropagation();
                          clearExtraOptions();
                        }}
                      />
                    </Badge>
                  )}
                </div>
                <div className="flex items-center">
                  <div
                    role="button"
                    tabIndex={0}
                    onClick={(e) => {
                      e.stopPropagation();
                      handleClear();
                    }}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault();
                        e.stopPropagation();
                        handleClear();
                      }
                    }}
                    className="mx-2 flex cursor-pointer items-center text-muted-foreground hover:text-foreground"
                  >
                    <XIcon className="h-4 w-4" />
                  </div>
                  <Separator orientation="vertical" className="flex min-h-6 h-full" />
                  <ChevronDown className="mx-2 h-4 w-4 cursor-pointer text-muted-foreground" />
                </div>
              </div>
            ) : (
              <div className="mx-auto flex w-full items-center justify-between">
                <span className="mx-3 text-sm text-muted-foreground">{placeholder}</span>
                <ChevronDown className="mx-2 h-4 w-4 cursor-pointer text-muted-foreground" />
              </div>
            )}
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-auto min-w-[200px] p-0" align="start">
          <Command>
            {searchable && (
              <CommandInput
                placeholder="Search..."
                value={searchValue}
                onValueChange={setSearchValue}
              />
            )}
            <CommandList className="max-h-[300px]">
              <CommandEmpty>No results found.</CommandEmpty>
              {!hideSelectAll && !searchValue && (
                <CommandGroup>
                  <CommandItem onSelect={toggleAll} className="cursor-pointer">
                    <div
                      className={cn(
                        'mr-2 flex h-4 w-4 items-center justify-center rounded-sm border border-primary',
                        selectedValues.length === options.length
                          ? 'bg-primary text-primary-foreground'
                          : 'opacity-50 [&_svg]:invisible',
                      )}
                    >
                      <CheckIcon className="h-4 w-4" />
                    </div>
                    <span>(Select All)</span>
                  </CommandItem>
                </CommandGroup>
              )}
              <CommandGroup>
                {filteredOptions.map((option) => {
                  const isSelected = selectedValues.includes(option.value);
                  return (
                    <CommandItem
                      key={option.value}
                      onSelect={() => toggleOption(option.value)}
                      className="cursor-pointer"
                    >
                      <div
                        className={cn(
                          'mr-2 flex h-4 w-4 items-center justify-center rounded-sm border border-primary',
                          isSelected
                            ? 'bg-primary text-primary-foreground'
                            : 'opacity-50 [&_svg]:invisible',
                        )}
                      >
                        <CheckIcon className="h-4 w-4" />
                      </div>
                      {option.icon && (
                        <option.icon className="mr-2 h-4 w-4 text-muted-foreground" />
                      )}
                      <span>{option.label}</span>
                    </CommandItem>
                  );
                })}
              </CommandGroup>
              <CommandSeparator />
              <CommandGroup>
                <div className="flex items-center justify-between">
                  {selectedValues.length > 0 && (
                    <>
                      <CommandItem
                        onSelect={handleClear}
                        className="flex-1 cursor-pointer justify-center"
                      >
                        Clear
                      </CommandItem>
                      <Separator orientation="vertical" className="flex min-h-6 h-full" />
                    </>
                  )}
                  <CommandItem
                    onSelect={() => setIsPopoverOpen(false)}
                    className="max-w-full flex-1 cursor-pointer justify-center"
                  >
                    Close
                  </CommandItem>
                </div>
              </CommandGroup>
            </CommandList>
          </Command>
        </PopoverContent>
      </Popover>
    );
  },
);

MultiSelect.displayName = 'MultiSelect';
export { MultiSelect };
export type { MultiSelectOption, MultiSelectProps };
