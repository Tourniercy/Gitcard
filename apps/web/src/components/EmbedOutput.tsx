import { useState, useCallback } from 'react';
import { Copy, Check } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { CardType } from '@/hooks/useCardConfig';

interface CardEmbedProps {
  card: CardType;
  buildSrc: (card: CardType) => string;
}

const CARD_ALT: Record<CardType, string> = {
  stats: 'GitHub Stats',
  streak: 'GitHub Streak',
  'top-langs': 'Top Languages',
  profile: 'GitHub Profile',
};

const API_BASE = import.meta.env.VITE_API_URL ?? '';

export function CardEmbed({ card, buildSrc }: CardEmbedProps) {
  const [copied, setCopied] = useState(false);

  const src = `${API_BASE}${buildSrc(card)}`;
  const markdown = `![${CARD_ALT[card]}](${src})`;

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(markdown);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // clipboard API may not be available
    }
  }, [markdown]);

  return (
    <div className="absolute top-2 right-2">
      <Button variant="outline" size="sm" className="h-7 gap-1.5 text-xs" onClick={handleCopy}>
        {copied ? <Check className="h-3 w-3" /> : <Copy className="h-3 w-3" />}
        {copied ? 'Copied!' : 'Copy Markdown'}
      </Button>
    </div>
  );
}
