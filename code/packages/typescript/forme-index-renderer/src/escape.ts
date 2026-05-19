/**
 * escape.ts — HTML escaping + URL validation.
 *
 * Same patterns as `forme-feeds` / `forme-opengraph`:
 *   - All five HTML entities (& < > " ') in a single-pass replace
 *   - ASCII control bytes stripped first
 *
 * URL policy: this package targets internal site URLs (the items in
 * an index page link to other pages on the same site).  We accept:
 *   - absolute http(s)://  URLs (cross-origin links to other sites)
 *   - root-relative /path  URLs (the common case for blog archives)
 * and reject everything else (javascript:, data:, file:, schemes
 * with embedded `:`).
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

/** Escape for HTML attribute value. */
export function escapeHtmlAttr(s: string): string {
  return s.replace(ASCII_CONTROL_RE, "").replace(HTML_ATTR_SPECIAL_RE, (ch) => HTML_ATTR_ESCAPE_MAP[ch]!);
}

/** Escape for HTML element text content. */
export function escapeHtmlText(s: string): string {
  return escapeHtmlAttr(s);
}

// ─── URL validation ─────────────────────────────────────────────────────

/**
 * Allowed URL forms for `item.url`:
 *   - absolute http(s):  ^https?://
 *   - root-relative:     ^/[^/]   (single leading slash; rejects `//` protocol-relative)
 *
 * Rejected:
 *   - javascript:, data:, file:, vbscript:, etc.
 *   - protocol-relative `//example.com/...`
 *   - bare relative `foo` (ambiguous in archives — caller should
 *     normalise to root-relative)
 *   - empty string / non-string
 */
const ABSOLUTE_HTTP_URL_RE = /^https?:\/\//i;
const ROOT_RELATIVE_URL_RE = /^\/[^/]/;
const ROOT_ALONE_RE        = /^\/$/;

export function assertItemUrl(value: unknown): void {
  if (typeof value !== "string" || value.length === 0) {
    throw new TypeError(
      `forme-index-renderer: item.url must be a non-empty string (got ${JSON.stringify(value)})`,
    );
  }
  if (
    !ABSOLUTE_HTTP_URL_RE.test(value)
    && !ROOT_RELATIVE_URL_RE.test(value)
    && !ROOT_ALONE_RE.test(value)
  ) {
    throw new TypeError(
      `forme-index-renderer: item.url must be absolute http(s) or root-relative "/path" (got ${JSON.stringify(value)})`,
    );
  }
}
