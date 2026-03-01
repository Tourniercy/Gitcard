import { useMemo } from 'react';
import parse from 'html-react-parser';
import {
  renderStatsCard,
  renderStreakCard,
  renderLangsCard,
  renderProfileCard,
  type CardOptions,
} from '@gitcard/svg-renderer';
import type { CardType } from '@/hooks/useCardConfig';
import type { GitHubData } from '@/hooks/useGitHubData';

interface CardPreviewProps {
  id: CardType;
  data: GitHubData;
  options: CardOptions;
}

const renderers: Record<CardType, (data: GitHubData, options: CardOptions) => string> = {
  stats: (data, options) => renderStatsCard(data.stats, options),
  streak: (data, options) => renderStreakCard(data.streak, options),
  'top-langs': (data, options) => renderLangsCard(data.languages, options),
  profile: (data, options) => (data.profile ? renderProfileCard(data.profile, options) : ''),
};

const CARD_WIDTHS: Record<CardType, number> = {
  stats: 495,
  streak: 495,
  'top-langs': 495,
  profile: 700,
};

function getMaxWidth(id: CardType, options: CardOptions): number {
  if (id === 'top-langs' && options.layout === 'compact') return 300;
  return CARD_WIDTHS[id];
}

export function CardPreview({ id, data, options }: CardPreviewProps) {
  const elements = useMemo(() => parse(renderers[id](data, options)), [id, data, options]);

  if (!elements) return null;

  return (
    <div className="p-4 [&>svg]:w-full" style={{ maxWidth: getMaxWidth(id, options) }}>
      {elements}
    </div>
  );
}
