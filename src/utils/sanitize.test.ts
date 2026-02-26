import { describe, expect, it } from 'vitest';
import { encodeHTML } from './sanitize';

describe('encodeHTML', () => {
  it('escapes ampersands', () => {
    expect(encodeHTML('a & b')).toBe('a &amp; b');
  });

  it('escapes angle brackets', () => {
    expect(encodeHTML('<script>')).toBe('&lt;script&gt;');
  });

  it('escapes double quotes', () => {
    expect(encodeHTML('"hello"')).toBe('&quot;hello&quot;');
  });

  it('escapes single quotes', () => {
    expect(encodeHTML("it's")).toBe('it&#39;s');
  });

  it('handles empty string', () => {
    expect(encodeHTML('')).toBe('');
  });

  it('handles string with no special chars', () => {
    expect(encodeHTML('hello world')).toBe('hello world');
  });

  it('escapes multiple special chars in one string', () => {
    expect(encodeHTML('<a href="x">&</a>')).toBe(
      '&lt;a href=&quot;x&quot;&gt;&amp;&lt;/a&gt;',
    );
  });
});
