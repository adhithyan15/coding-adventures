/**
 * validate.ts — route, baseUrl, and field validators.
 *
 * The route validator is path-traversal defence.  Routes
 * become file output paths via `routeToOutputPath`, so any
 * `..` segment or absolute prefix would let an attacker write
 * outside the deploy root.  We reject everything except a
 * narrow set of "safe" root-relative paths.
 *
 * @module validate
 */

// Segment charset: RFC 3986 unreserved (A-Z a-z 0-9 . _ ~ -) +
// sub-delims (! $ & ' ( ) * + , ; =) + colon + at-sign.
// IMPORTANT: `%` is deliberately NOT in this set — percent-
// encoded sequences like `%2e%2e` (a `..` traversal) or `%00`
// (NUL byte) would bypass the segment-by-segment `..` and
// control-byte checks below.  We reject `%` wholesale rather
// than try to decode and re-check.  Callers must pre-decode
// routes (or never percent-encode them in the first place;
// routes are internal config, not user-typed URLs).
const ROUTE_SEGMENT_RE = /^[A-Za-z0-9._~!$&'()*+,;=:@\-]+$/;

/**
 * Validate a route string.  Returns the canonicalised route
 * (currently identity — we don't lower-case or trim because
 * routes are case-sensitive URLs).
 *
 * Rejects:
 *   - non-string / empty / undefined / null
 *   - not starting with `/`
 *   - starting with `//` (protocol-relative)
 *   - starting with `/\` (backslash variant)
 *   - containing `\` anywhere (Windows path separator)
 *   - containing `..` as a segment (path traversal)
 *   - containing `.` as a sole segment (relative path)
 *   - containing an empty segment (`//` mid-path)
 *   - containing a segment with disallowed characters (only
 *     URL unreserved + sub-delims + `:`/`@` allowed; no `?`,
 *     no `#`, no whitespace, no NUL)
 *   - exceeding length 2048 (sanity cap)
 */
export function validateRoute(value: unknown, field: string): string {
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-page-bundle-emitter: ${field} must be a string; got ${
        value === null ? "null" : typeof value
      }`,
    );
  }
  if (value.length === 0) {
    throw new TypeError(
      `forme-aot-page-bundle-emitter: ${field} must be non-empty`,
    );
  }
  if (value.length > 2048) {
    throw new TypeError(
      `forme-aot-page-bundle-emitter: ${field} must be ≤ 2048 chars`,
    );
  }
  if (value[0] !== "/") {
    throw new TypeError(
      `forme-aot-page-bundle-emitter: ${field} must start with "/"; got ${JSON.stringify(shorten(value))}`,
    );
  }
  if (value === "/") return value;
  if (value[1] === "/") {
    throw new TypeError(
      `forme-aot-page-bundle-emitter: ${field} must not start with "//" (protocol-relative); got ${JSON.stringify(shorten(value))}`,
    );
  }
  if (value[1] === "\\") {
    throw new TypeError(
      `forme-aot-page-bundle-emitter: ${field} must not start with "/\\" (backslash variant); got ${JSON.stringify(shorten(value))}`,
    );
  }
  if (value.indexOf("\\") !== -1) {
    throw new TypeError(
      `forme-aot-page-bundle-emitter: ${field} must not contain "\\"; got ${JSON.stringify(shorten(value))}`,
    );
  }

  // Walk the segments.  Note: split on "/" yields the empty
  // string before the first "/" (which we discard); also
  // catches `//` mid-path (yields an empty interior segment).
  const segments = value.slice(1).split("/");
  for (const seg of segments) {
    if (seg.length === 0) {
      throw new TypeError(
        `forme-aot-page-bundle-emitter: ${field} must not contain empty segments (//); got ${JSON.stringify(shorten(value))}`,
      );
    }
    if (seg === "..") {
      throw new TypeError(
        `forme-aot-page-bundle-emitter: ${field} must not contain ".." segments (path traversal); got ${JSON.stringify(shorten(value))}`,
      );
    }
    if (seg === ".") {
      throw new TypeError(
        `forme-aot-page-bundle-emitter: ${field} must not contain "." segments; got ${JSON.stringify(shorten(value))}`,
      );
    }
    if (!ROUTE_SEGMENT_RE.test(seg)) {
      throw new TypeError(
        `forme-aot-page-bundle-emitter: ${field} segment ${JSON.stringify(seg)} contains disallowed characters; only [A-Za-z0-9._~!$&'()*+,;=:@-] permitted`,
      );
    }
  }
  return value;
}

/**
 * Validate `baseUrl` — http(s):// only.
 */
export function validateBaseUrl(value: unknown): string {
  if (typeof value !== "string" || value.length === 0) {
    throw new TypeError(
      `forme-aot-page-bundle-emitter: baseUrl must be a non-empty string; got ${
        value === null ? "null" : typeof value
      }`,
    );
  }
  if (value.length > 2048) {
    throw new TypeError(
      `forme-aot-page-bundle-emitter: baseUrl must be ≤ 2048 chars`,
    );
  }
  const head = value.slice(0, 8).toLowerCase();
  if (!head.startsWith("http://") && !head.startsWith("https://")) {
    throw new TypeError(
      `forme-aot-page-bundle-emitter: baseUrl must be http(s)://; got ${JSON.stringify(shorten(value))}`,
    );
  }
  return value;
}

/**
 * Validate that a value is a string (used for `html`,
 * `lastmod`, `contentType`).
 */
export function validateString(value: unknown, field: string): string {
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-page-bundle-emitter: ${field} must be a string; got ${
        value === null ? "null" : typeof value
      }`,
    );
  }
  return value;
}

function shorten(s: string): string {
  return s.length > 100 ? `${s.slice(0, 100)}…` : s;
}
