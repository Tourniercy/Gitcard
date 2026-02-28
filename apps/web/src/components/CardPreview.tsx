import { useMemo } from 'react';
import {
  renderStatsCard,
  renderStreakCard,
  renderLangsCard,
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
};

export function CardPreview({ id, data, options }: CardPreviewProps) {
  const svg = useMemo(() => renderers[id](data, options), [id, data, options]);
  return <div className="w-full max-w-[495px] p-4" dangerouslySetInnerHTML={{ __html: svg }} />;
}
