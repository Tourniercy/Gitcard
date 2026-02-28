import type { CardOptions } from '@gitcard/svg-renderer';
import type { CardType } from '@/hooks/useCardConfig';
import type { GitHubData } from '@/hooks/useGitHubData';
import { Separator } from '@/components/ui/separator';
import { CardPreview } from './CardPreview';
import { CardEmbed } from './EmbedOutput';

interface CardListProps {
  cards: CardType[];
  data: GitHubData;
  options: CardOptions;
  buildSrc: (card: CardType) => string;
}

export function CardList({ cards, data, options, buildSrc }: CardListProps) {
  return (
    <div className="flex flex-col gap-4">
      {cards.map((card) => (
        <div
          key={card}
          className="rounded-lg border bg-card shadow-sm overflow-hidden transition-shadow hover:shadow-md"
        >
          <CardPreview id={card} data={data} options={options} />
          <Separator />
          <CardEmbed card={card} buildSrc={buildSrc} />
        </div>
      ))}
    </div>
  );
}
