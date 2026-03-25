/**
 * URL Utilities for the HTML Sanitizer
 *
 * Independent copy of the URL scheme detection logic. This is intentionally
 * not shared with the AST sanitizer — the HTML sanitizer has no dependency on
 * document-ast (string in, string out), and sharing would create a transitive
 * dependency that violates the package architecture (Decision 1 from spec).
 *
 * The algorithm is identical to the AST sanitizer's url-utils:
 *   1. Strip C0 controls + zero-width chars
 *   2. Extract scheme (everything before first ":")
 *   3. Relative URLs always pass
 *
 * Spec: TE02 §URL Scheme Sanitization
 *
 * @module url-utils
 */

// ─── Control Character Pattern ────────────────────────────────────────────────
//
// Same characters as the AST sanitizer. See that module for detailed comments.
// Repeated here so this package has zero external dependencies.

const STRIP_PATTERN = /[\u0000-\u001F\u200B-\u200D\u2060\uFEFF]/gu;

/**
 * Strip C0 control characters and zero-width Unicode characters from a string.
 *
 * Prevents bypass attacks where control characters are inserted into a URL
 * scheme (e.g. "java\x00script:" → "javascript:" after stripping).
 */
export function stripControlChars(url: string): string {
  return url.replace(STRIP_PATTERN, "");
}

/**
 * Extract the URL scheme from a pre-stripped URL.
 *
 * Returns null for relative URLs (no scheme) or when ":" appears after
 * a path separator ("/" or "?").
 */
export function extractScheme(strippedUrl: string): string | null {
  if (!strippedUrl) return null;

  const colonIndex = strippedUrl.indexOf(":");
  if (colonIndex === -1) return null;

  const slashIndex = strippedUrl.indexOf("/");
  const queryIndex = strippedUrl.indexOf("?");

  if (slashIndex !== -1 && slashIndex < colonIndex) return null;
  if (queryIndex !== -1 && queryIndex < colonIndex) return null;

  return strippedUrl.slice(0, colonIndex).toLowerCase();
}

/**
 * Check whether a URL is permitted by the given allowlist.
 *
 * @param url               Raw URL value from an HTML attribute.
 * @param allowedUrlSchemes Allowed schemes, or null to allow all.
 * @returns                 true if the URL is safe; false to reject.
 */
export function isSchemeAllowed(
  url: string,
  allowedUrlSchemes: readonly string[] | null | undefined,
): boolean {
  if (allowedUrlSchemes === null || allowedUrlSchemes === undefined) return true;
  const stripped = stripControlChars(url);
  const scheme = extractScheme(stripped);
  if (scheme === null) return true; // relative URL — always safe
  return (allowedUrlSchemes as readonly string[]).includes(scheme);
}
