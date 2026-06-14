/**
 * escape.ts — HTML attribute escaping.  Same single-pass
 * character-class pattern as the sibling FM00 emitters.
 *
 * @module escape
 */

const HTML_ATTR_ESCAPE_MAP: Readonly<Record<string, string>> = Object.freeze({
  "&": "&amp;",
  "<": "&lt;",
  ">": "&gt;",
  "\"": "&quot;",
  "'": "&#39;",
});

const HTML_ATTR_SPECIAL_RE = /[&<>"']/g;
// eslint-disable-next-line no-control-regex
const ASCII_CONTROL_RE = /[\x00-\x1F\x7F]/g;

export function stripAsciiControl(s: string): string {
  return String(s).replace(ASCII_CONTROL_RE, "");
}

export function escapeHtmlAttr(s: string): string {
  return stripAsciiControl(s).replace(HTML_ATTR_SPECIAL_RE, (ch) => HTML_ATTR_ESCAPE_MAP[ch]!);
}
