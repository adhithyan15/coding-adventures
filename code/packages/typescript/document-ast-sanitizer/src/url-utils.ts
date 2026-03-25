/**
 * URL Utilities for the AST Sanitizer
 *
 * URL scheme detection is surprisingly subtle. Browsers silently strip
 * certain control characters before parsing a URL scheme, enabling bypasses
 * like `java\x00script:` which a naive string comparison would pass.
 *
 * This module implements the detection algorithm from the TE02 spec:
 *
 *   Step 1: Strip C0 controls + zero-width characters
 *   Step 2: Extract the scheme (everything before the first ":")
 *   Step 3: Lowercase and check against the allowlist
 *   Step 4: Relative URLs (no scheme present) always pass
 *
 * The key insight is that **relative URLs are always safe** — they resolve
 * against the current page's origin, so they can never point to a different
 * origin or execute code via a dangerous scheme.
 *
 * Spec: TE02 — Document Sanitization §URL Scheme Sanitization
 *
 * @module url-utils
 */

// ─── Control Character Patterns ──────────────────────────────────────────────
//
// These are the exact characters that WHATWG URL parsers strip before scheme
// detection. By stripping them first, we prevent bypasses where an attacker
// inserts invisible characters to fool a string match.
//
// Characters stripped:
//
//   U+0000–U+001F   C0 controls (NUL, TAB, LF, CR, ESC, etc.)
//                   Browsers remove these silently from URL schemes.
//                   "java\rscript:" → "javascript:" in WHATWG parsers.
//
//   U+200B          ZERO WIDTH SPACE
//   U+200C          ZERO WIDTH NON-JOINER
//   U+200D          ZERO WIDTH JOINER
//   U+2060          WORD JOINER
//   U+FEFF          BOM / ZERO WIDTH NO-BREAK SPACE
//
//   These Unicode "invisible" characters can also be silently ignored by
//   some parsers, enabling bypasses like "\u200bjavascript:".
//
// Why not strip U+007F–U+009F?
//   The AST sanitizer operates on already-decoded text (the CommonMark parser
//   decodes character references before building the AST). C1 controls in raw
//   decoded text are unusual and we follow the minimal WHATWG set here. The
//   HTML sanitizer (document-html-sanitizer) takes a wider approach.

const STRIP_PATTERN = /[\u0000-\u001F\u200B-\u200D\u2060\uFEFF]/gu;

/**
 * Strip C0 control characters and zero-width Unicode characters from a string.
 *
 * This is the first step in URL scheme extraction. Browsers silently ignore
 * these characters when parsing URL schemes, so we must remove them before
 * attempting scheme detection to prevent bypass attacks.
 *
 * @example
 * stripControlChars("java\x00script:")   // → "javascript:"
 * stripControlChars("java\rscript:")     // → "javascript:"
 * stripControlChars("\u200bjavascript:") // → "javascript:"
 * stripControlChars("https://ok.com")   // → "https://ok.com" (unchanged)
 */
export function stripControlChars(url: string): string {
  return url.replace(STRIP_PATTERN, "");
}

/**
 * Extract the URL scheme from a pre-stripped URL string.
 *
 * The scheme is the part before the first ":". Returns null for relative URLs
 * (no ":" present, or ":" appears after a "/" or "?").
 *
 * Why the "/" and "?" check?
 *   A relative URL like "/path:value" or "?key:val" contains a ":" but it's
 *   not a scheme separator — it's part of the path or query. A real scheme
 *   always appears before any path separator.
 *
 * Truth table:
 *
 *   URL                      | Scheme extracted
 *   ─────────────────────────┼─────────────────
 *   "https://example.com"    | "https"
 *   "javascript:alert(1)"   | "javascript"
 *   "mailto:user@host"      | "mailto"
 *   "/relative/path"        | null (no ":")
 *   "path/to/page"          | null (no ":")
 *   "?query=string"         | null (no ":")
 *   "/path:with:colons"     | null (":" after "/")
 *   "?q=a:b"                | null (":" after "?")
 *   ""                      | null (empty)
 *
 * @param strippedUrl  URL with control characters already removed.
 * @returns            Lowercase scheme string, or null for relative URLs.
 */
export function extractScheme(strippedUrl: string): string | null {
  if (!strippedUrl) return null;

  const colonIndex = strippedUrl.indexOf(":");
  if (colonIndex === -1) {
    // No colon at all — definitely a relative URL
    return null;
  }

  // Check if the colon appears after a path separator.
  // If "/" or "?" appears before the first ":", this is a relative URL
  // with a colon in the path/query, not a scheme.
  const slashIndex = strippedUrl.indexOf("/");
  const queryIndex = strippedUrl.indexOf("?");

  if (slashIndex !== -1 && slashIndex < colonIndex) return null;
  if (queryIndex !== -1 && queryIndex < colonIndex) return null;

  // Everything before the colon is the scheme. Lowercase for comparison.
  return strippedUrl.slice(0, colonIndex).toLowerCase();
}

/**
 * Check whether a URL is allowed by the given scheme allowlist.
 *
 * This is the entry point for the URL policy check. It combines the two
 * steps (strip control chars, extract scheme) and applies the allowlist.
 *
 * @param url                The raw URL from the AST node.
 * @param allowedUrlSchemes  List of allowed schemes, or null to allow all.
 * @returns                  true if the URL is safe to use, false to reject it.
 *
 * @example
 * isSchemeAllowed("https://ok.com", ["http", "https"])  // → true
 * isSchemeAllowed("javascript:x",   ["http", "https"])  // → false
 * isSchemeAllowed("/relative",      ["http", "https"])  // → true  (relative always ok)
 * isSchemeAllowed("anything",       null)               // → true  (null = allow all)
 */
export function isSchemeAllowed(
  url: string,
  allowedUrlSchemes: readonly string[] | null | undefined,
): boolean {
  // null means "allow everything" (PASSTHROUGH policy)
  if (allowedUrlSchemes === null || allowedUrlSchemes === undefined) {
    return true;
  }

  const stripped = stripControlChars(url);
  const scheme = extractScheme(stripped);

  // Relative URLs (no scheme) always pass through.
  // This is safe because relative URLs resolve against the current origin.
  if (scheme === null) {
    return true;
  }

  // Check if the extracted scheme is in the allowlist.
  // Scheme is already lowercased by extractScheme.
  return (allowedUrlSchemes as readonly string[]).includes(scheme);
}
