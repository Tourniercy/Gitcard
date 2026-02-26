import { z } from 'zod';
import type { CardOptions } from '../types.js';

const booleanString = z
  .enum(['true', 'false', '1', '0', ''])
  .transform((v) => v === 'true' || v === '1')
  .default('true');

const booleanStringFalse = z
  .enum(['true', 'false', '1', '0', ''])
  .transform((v) => v === 'true' || v === '1')
  .default('false');

const querySchema = z.object({
  theme: z.string().default('default'),
  hide: z
    .string()
    .default('')
    .transform((v) => (v ? v.split(',').map((s) => s.trim()) : [])),
  show_icons: booleanString,
  hide_border: booleanStringFalse,
  hide_title: booleanStringFalse,
  bg_color: z.string().optional(),
  title_color: z.string().optional(),
  text_color: z.string().optional(),
  icon_color: z.string().optional(),
  border_color: z.string().optional(),
  cache_seconds: z.coerce
    .number()
    .catch(14400)
    .transform((v) => Math.max(v, 1800))
    .default(14400),
  locale: z.string().default('en'),
});

export function parseCardOptions(query: Record<string, string | undefined>): CardOptions {
  const parsed = querySchema.parse(query);
  return {
    theme: parsed.theme,
    hide: parsed.hide,
    showIcons: parsed.show_icons,
    hideBorder: parsed.hide_border,
    hideTitle: parsed.hide_title,
    bgColor: parsed.bg_color,
    titleColor: parsed.title_color,
    textColor: parsed.text_color,
    iconColor: parsed.icon_color,
    borderColor: parsed.border_color,
    cacheSeconds: parsed.cache_seconds,
    locale: parsed.locale,
  };
}
