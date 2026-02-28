import { useMemo } from 'react';
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
  profile: (data, options) => renderProfileCard(data.profile, options),
};

export function CardPreview({ id, data, options }: CardPreviewProps) {
  const svg = useMemo(() => renderers[id](data, options), [id, data, options]);
  const isCompactLangs = id === 'top-langs' && options.layout === 'compact';
  const maxWidth = id === 'profile' ? '' : isCompactLangs ? 'max-w-[300px]' : 'max-w-[495px]';
  return <div className={`w-full ${maxWidth} p-4`} dangerouslySetInnerHTML={{ __html: svg }} />;
}
