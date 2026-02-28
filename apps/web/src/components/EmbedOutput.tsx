import { useState, useCallback } from 'react';
import { Copy, Check } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import type { CardType } from '@/hooks/useCardConfig';

interface EmbedOutputProps {
  cards: CardType[];
  buildSrc: (card: CardType) => string;
}

const CARD_ALT: Record<CardType, string> = {
  stats: 'GitHub Stats',
  streak: 'GitHub Streak',
  'top-langs': 'Top Languages',
};

const API_BASE = import.meta.env.VITE_API_URL ?? '';

export function EmbedOutput({ cards, buildSrc }: EmbedOutputProps) {
  const [copiedType, setCopiedType] = useState<string | null>(null);

  const markdownLines = cards.map((card) => {
    const src = `${API_BASE}${buildSrc(card)}`;
    return `![${CARD_ALT[card]}](${src})`;
  });

  const htmlLines = cards.map((card) => {
    const src = `${API_BASE}${buildSrc(card)}`;
    return `<img src="${src}" alt="${CARD_ALT[card]}" />`;
  });

  const markdown = markdownLines.join('\n');
  const html = htmlLines.join('\n');

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
    <div className="flex flex-col gap-4">
      <div>
        <div className="flex items-center justify-between mb-2">
          <span className="text-sm font-medium">Markdown</span>
          <Button variant="ghost" size="sm" onClick={() => handleCopy(markdown, 'markdown')}>
            {copiedType === 'markdown' ? (
              <>
                <Check className="mr-1 h-3 w-3" />
                Copied
              </>
            ) : (
              <>
                <Copy className="mr-1 h-3 w-3" />
                Copy
              </>
            )}
          </Button>
        </div>
        <pre className="rounded-md bg-muted p-3 text-xs font-mono overflow-x-auto">
          <code>{markdown}</code>
        </pre>
      </div>

      <Separator />

      <div>
        <div className="flex items-center justify-between mb-2">
          <span className="text-sm font-medium">HTML</span>
          <Button variant="ghost" size="sm" onClick={() => handleCopy(html, 'html')}>
            {copiedType === 'html' ? (
              <>
                <Check className="mr-1 h-3 w-3" />
                Copied
              </>
            ) : (
              <>
                <Copy className="mr-1 h-3 w-3" />
                Copy
              </>
            )}
          </Button>
        </div>
        <pre className="rounded-md bg-muted p-3 text-xs font-mono overflow-x-auto">
          <code>{html}</code>
        </pre>
      </div>
    </div>
  );
}
