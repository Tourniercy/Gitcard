import type { CardOptions } from '@gitcard/svg-renderer';
import type { CardType } from '@/hooks/useCardConfig';
import type { GitHubData } from '@/hooks/useGitHubData';
import { CardPreview } from './CardPreview';
import { CardEmbed } from './EmbedOutput';

interface CardListProps {
  cards: CardType[];
  data: GitHubData;
  options: CardOptions;
  buildSrc: (card: CardType) => string;
  gridClassName?: string;
}

export function CardList({ cards, data, options, buildSrc, gridClassName }: CardListProps) {
  return (
    <div className={gridClassName ?? 'flex flex-col gap-4'}>
      {cards.map((card) => (
        <div
          key={card}
          className="relative rounded-lg border bg-card shadow-sm overflow-hidden transition-shadow hover:shadow-md"
        >
          <CardPreview id={card} data={data} options={options} />
          <CardEmbed card={card} buildSrc={buildSrc} />
        </div>
      ))}
    </div>
  );
}
