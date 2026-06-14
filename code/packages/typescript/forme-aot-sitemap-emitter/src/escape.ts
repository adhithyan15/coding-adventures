/**
 * escape.ts — XML escaping + invalid-character stripping.
 *
 * Same pattern as `forme-feeds` — escape all five XML 1.0
 * predefined entities (`& < > " '`) in element text and
 * attribute values via a single-pass character-class
 * replacement (CodeQL-friendly form, no
 * "incomplete-string-escape" warning).  Strip C0 control bytes
 * that XML 1.0 §2.2 forbids (everything in `0x00-0x1F` except
 * `\t \n \r`).
 *
 * Why the duplicate copy in this package rather than depending
 * on `forme-feeds`?  Sitemap emission is conceptually
 * downstream of feeds — both produce XML, but sitemap callers
 * don't need an RSS/Atom dependency.  The 30-line escape
 * helper is small enough that duplication is cheaper than a
 * cross-package import.
 *
 * @module escape
 */

const XML_ESCAPE_MAP: Readonly<Record<string, string>> = Object.freeze({
  "&": "&amp;",
  "<": "&lt;",
  ">": "&gt;",
  "\"": "&quot;",
  "'": "&apos;",
});

// All five XML predefined entities — single-pass replacement.
const XML_SPECIAL_RE = /[&<>"']/g;

// XML 1.0 §2.2 forbidden C0 controls except \t \n \r.
// Pattern: NUL to backspace, vertical-tab + form-feed, SO onwards.
// eslint-disable-next-line no-control-regex
const INVALID_XML_RE = /[\x00-\x08\x0B\x0C\x0E-\x1F]/g;

/**
 * Strip XML 1.0-illegal C0 control characters.  Returns a fresh
 * string with the forbidden bytes removed; `\t \n \r` preserved.
 */
export function stripInvalidXml(s: string): string {
  return String(s).replace(INVALID_XML_RE, "");
}

/**
 * Escape XML special characters for use in element text or
 * attribute value.  Strips invalid XML chars first so the
 * output is always well-formed XML 1.0.
 */
export function escapeXml(s: string): string {
  return stripInvalidXml(s).replace(XML_SPECIAL_RE, (ch) => XML_ESCAPE_MAP[ch]!);
}
