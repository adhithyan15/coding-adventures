/**
 * escape.ts — HTML attribute escaping + URL scheme validation.
 *
 * Two concerns:
 *
 * 1. **HTML attribute-value escaping.**  All meta-tag content
 *    values land inside `content="..."` / `href="..."` attributes.
 *    The five XHTML predefined entities (`& < > " '`) must be
 *    escaped — `"` and `'` to keep the attribute literal closed,
 *    `&` because it can introduce a different entity, and `< >`
 *    for parser safety even though strict HTML5 only requires `&`
 *    + the active quote character.  Single-pass replace via a
 *    character-class regex (CodeQL incomplete-string-escape rule
 *    accepts this form).
 *
 * 2. **URL scheme validation.**  `og:image`, `og:url`, `og:video`,
 *    `twitter:image`, and `<link rel="canonical">` MUST be ABSOLUTE
 *    URLs per the respective specs.  We additionally enforce that
 *    the scheme is `http://` or `https://` — `javascript:` and
 *    `data:` URLs in social-card meta tags are an injection vector
 *    (a scraper rendering the URL in a preview could execute JS in
 *    its own origin, or load a tracker via `data:`).
 *
 * ASCII control bytes (0x00–0x1F, 0x7F) are stripped from every
 * string before escaping — they have no legitimate place in a
 * meta-tag value and they can confuse downstream HTML parsers.
 *
 * @module escape
 */

const HTML_ATTR_ESCAPE_MAP: Readonly<Record<string, string>> = Object.freeze({
  "&": "&amp;",
  "<": "&lt;",
  ">": "&gt;",
  "\"": "&quot;",
  "'": "&#39;",   // &#39; is more portable than &apos; (older HTML4 parsers)
});

// Single-pass replacement (CodeQL incomplete-string-escape friendly).
const HTML_ATTR_SPECIAL_RE = /[&<>"']/g;

// ASCII control bytes + DEL.  Excludes the high-bit C1 range
// (0x80–0x9F) — those are valid Unicode code points in HTML5 (text
// content is UTF-8) and shouldn't be stripped from meta-tag values.
// eslint-disable-next-line no-control-regex
const ASCII_CONTROL_RE = /[\x00-\x1F\x7F]/g;

/**
 * Escape a string for use inside an HTML attribute value (double-
 * quoted).  Strips ASCII control bytes first.
 */
export function escapeHtmlAttr(s: string): string {
  return s.replace(ASCII_CONTROL_RE, "").replace(HTML_ATTR_SPECIAL_RE, (ch) => HTML_ATTR_ESCAPE_MAP[ch]!);
}

/**
 * Escape a string for use inside an HTML text node (e.g. inside
 * `<title>`).  Same rules apply — we escape all five entities for
 * uniformity even though `<title>` content is RCDATA (only `&` and
 * `<` strictly need escaping there).
 */
export function escapeHtmlText(s: string): string {
  return escapeHtmlAttr(s);
}

// ─── URL validation ─────────────────────────────────────────────────────

/**
 * Allowed URL schemes for meta-tag fields that resolve to a fetch
 * target (og:image, og:url, og:video, twitter:image, canonical).
 * `javascript:` and `data:` are explicitly rejected — they're
 * injection vectors in social-card scrapers that render previews.
 */
const ABSOLUTE_HTTP_URL_RE = /^https?:\/\//i;

/**
 * Assert that a URL is an absolute http(s) URL.  Throws `TypeError`
 * on anything else (relative path, `javascript:`, `data:`, `file:`,
 * empty string, non-string input).  The error message includes the
 * field name to help callers diagnose which input failed.
 */
export function assertAbsoluteUrl(field: string, value: unknown): void {
  if (typeof value !== "string" || value.length === 0) {
    throw new TypeError(
      `forme-opengraph: ${field} must be a non-empty string (got ${JSON.stringify(value)})`,
    );
  }
  if (!ABSOLUTE_HTTP_URL_RE.test(value)) {
    throw new TypeError(
      `forme-opengraph: ${field} must be an absolute http(s) URL (got ${JSON.stringify(value)})`,
    );
  }
}
