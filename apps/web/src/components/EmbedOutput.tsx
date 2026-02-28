import { useState, useCallback } from 'react';
import type { CardType } from '../hooks/useCardConfig';

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
    <div className="embed-output">
      <h3>Embed Code</h3>

      <div className="embed-section">
        <div className="embed-header">
          <span className="embed-label">Markdown</span>
          <button
            type="button"
            className="copy-btn"
            onClick={() => handleCopy(markdown, 'markdown')}
          >
            {copiedType === 'markdown' ? 'Copied!' : 'Copy Markdown'}
          </button>
        </div>
        <pre className="embed-code">
          <code>{markdown}</code>
        </pre>
      </div>

      <div className="embed-section">
        <div className="embed-header">
          <span className="embed-label">HTML</span>
          <button type="button" className="copy-btn" onClick={() => handleCopy(html, 'html')}>
            {copiedType === 'html' ? 'Copied!' : 'Copy HTML'}
          </button>
        </div>
        <pre className="embed-code">
          <code>{html}</code>
        </pre>
      </div>
    </div>
  );
}
