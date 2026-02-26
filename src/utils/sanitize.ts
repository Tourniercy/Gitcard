const ESCAPE_MAP: Record<string, string> = {
  '&': '&amp;',
  '<': '&lt;',
  '>': '&gt;',
  '"': '&quot;',
  "'": '&#39;',
};

const ESCAPE_RE = /[&<>"']/g;

export function encodeHTML(str: string): string {
  return str.replace(ESCAPE_RE, (char) => ESCAPE_MAP[char]!);
}
