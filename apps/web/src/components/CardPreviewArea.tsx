import { useCardConfigContext } from '@/components/CardConfigProvider';
import { CardList } from '@/components/CardList';
import { Skeleton } from '@/components/ui/skeleton';

interface CardPreviewAreaProps {
  className?: string;
  gridClassName?: string;
}

export function CardPreviewArea({ className, gridClassName }: CardPreviewAreaProps) {
  const { config, data, isPending, error, allCards, cardOptions, buildSrc } =
    useCardConfigContext();

  if (!config.username.trim()) {
    return (
      <div className={className}>
        <div className="flex min-h-[300px] items-center justify-center rounded-lg border-2 border-dashed">
          <p className="text-muted-foreground">Enter a GitHub username to get started</p>
        </div>
      </div>
    );
  }

  if (isPending) {
    return (
      <div className={className}>
        <div className={gridClassName ?? 'flex flex-col gap-4'}>
          {allCards.map((card) => (
            <div key={card} className="rounded-lg border bg-card shadow-sm overflow-hidden">
              <Skeleton className="h-[195px] w-full rounded-none" />
              <div className="flex items-center gap-2 border-t p-3">
                <Skeleton className="h-4 w-20" />
                <Skeleton className="ml-auto h-8 w-24 rounded-md" />
              </div>
            </div>
          ))}
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className={className}>
        <div className="flex min-h-[300px] items-center justify-center rounded-lg border-2 border-dashed border-destructive bg-destructive/5">
          <p className="text-destructive">{error.message}</p>
        </div>
      </div>
    );
  }

  if (!data) return null;

  return (
    <div className={className}>
      <CardList
        cards={allCards}
        data={data}
        options={cardOptions}
        buildSrc={buildSrc}
        gridClassName={gridClassName}
      />
    </div>
  );
}
