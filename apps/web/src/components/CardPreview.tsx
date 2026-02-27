import type { CardType } from '../hooks/useCardConfig';

interface CardPreviewProps {
  id: CardType;
  src: string;
}

const CARD_ALT: Record<CardType, string> = {
  stats: 'GitHub Stats',
  streak: 'GitHub Streak',
  'top-langs': 'Top Languages',
};

export function CardPreview({ id, src }: CardPreviewProps) {
  return <img className="card-preview-img" src={src} alt={CARD_ALT[id]} width={495} />;
}
