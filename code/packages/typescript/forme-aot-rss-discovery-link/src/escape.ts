/**
 * escape.ts — HTML attribute escaping.
 *
 * Same single-pass character-class replacement pattern used by
 * `forme-feeds` / `forme-opengraph` / `forme-index-renderer`.
 * Covers all five HTML entities in a CodeQL-friendly form
 * (no incomplete-string-escape warning).  Strips ASCII control
 * bytes first so attribute values are guaranteed safe to
 * interpolate.
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

/**
 * Strip ASCII control bytes (`\x00-\x1F`, `\x7F`).  Returns a
 * fresh string.
 */
export function stripAsciiControl(s: string): string {
  return String(s).replace(ASCII_CONTROL_RE, "");
}

/**
 * Escape HTML attribute special chars + strip control bytes.
 * Single-pass character-class replace.
 */
export function escapeHtmlAttr(s: string): string {
  return stripAsciiControl(s).replace(HTML_ATTR_SPECIAL_RE, (ch) => HTML_ATTR_ESCAPE_MAP[ch]!);
}
