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
  const [copiedType, setCopiedType] = useState<string | null>(null);

  const src = `${API_BASE}${buildSrc(card)}`;
  const markdown = `![${CARD_ALT[card]}](${src})`;
  const html = `<img src="${src}" alt="${CARD_ALT[card]}" />`;

  const handleCopy = useCallback(async (text: string, type: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopiedType(type);
      setTimeout(() => setCopiedType(null), 2000);
    } catch {
      // clipboard API may not be available
    }
  }, []);

  return (
    <div className="flex flex-col gap-2 px-4 pb-4">
      <div className="flex gap-2">
        <pre className="flex-1 rounded-md bg-muted p-2 text-xs font-mono overflow-x-auto">
          <code>{markdown}</code>
        </pre>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8 shrink-0"
          onClick={() => handleCopy(markdown, `md-${card}`)}
          title="Copy Markdown"
        >
          {copiedType === `md-${card}` ? (
            <Check className="h-3.5 w-3.5" />
          ) : (
            <Copy className="h-3.5 w-3.5" />
          )}
        </Button>
      </div>
      <div className="flex gap-2">
        <pre className="flex-1 rounded-md bg-muted p-2 text-xs font-mono overflow-x-auto">
          <code>{html}</code>
        </pre>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8 shrink-0"
          onClick={() => handleCopy(html, `html-${card}`)}
          title="Copy HTML"
        >
          {copiedType === `html-${card}` ? (
            <Check className="h-3.5 w-3.5" />
          ) : (
            <Copy className="h-3.5 w-3.5" />
          )}
        </Button>
      </div>
    </div>
  );
}
