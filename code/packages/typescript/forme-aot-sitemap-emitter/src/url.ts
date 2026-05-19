/**
 * url.ts — URL resolution and validation.
 *
 * Sitemap entries reach the emitter as one of two shapes:
 *
 *   1. Absolute `http(s)://...` — used verbatim.
 *   2. Root-relative `/path` — joined with the caller's
 *      `baseUrl` (which must itself be `http(s)://`).
 *
 * Anything else — `javascript:`, `data:`, `file:`,
 * protocol-relative `//host`, bare relative `about`, empty
 * string, non-string — throws `TypeError` synchronously BEFORE
 * any XML is emitted.  This is the security chokepoint:
 * downstream renderers / crawlers blindly trust whatever URL
 * we put in `<loc>`, so the emitter is the line of defence.
 *
 * @module url
 */

/**
 * Trim a trailing slash from `baseUrl` so we can splice in
 * root-relative paths without doubling slashes.  Validates that
 * `baseUrl` itself is `http(s)://`-shaped — anything else
 * throws.
 */
export function normaliseBaseUrl(baseUrl: string): string {
  if (typeof baseUrl !== "string" || baseUrl.length === 0) {
    throw new TypeError(
      `forme-aot-sitemap-emitter: baseUrl must be a non-empty string; got ${
        baseUrl === null ? "null" : typeof baseUrl
      }`,
    );
  }
  if (!isHttpUrl(baseUrl)) {
    throw new TypeError(
      `forme-aot-sitemap-emitter: baseUrl must be http(s)://; got ${
        JSON.stringify(baseUrl.length > 100 ? baseUrl.slice(0, 100) + "…" : baseUrl)
      }`,
    );
  }
  // Strip exactly one trailing slash if present so "/about" +
  // "https://x.com/" doesn't become "https://x.com//about".
  return baseUrl.endsWith("/") ? baseUrl.slice(0, -1) : baseUrl;
}

/**
 * Resolve a single entry URL against `baseUrl`.
 *
 * Accept set:
 *   - http(s):// — returned as-is.
 *   - `/path` (root-relative, NOT `//host`) — returned as
 *     `baseUrl + path`.
 *   - `/` exactly — returned as `baseUrl + "/"`.
 *
 * Reject set (throws `TypeError`):
 *   - `javascript:`, `data:`, `file:`, `mailto:`, etc.
 *   - `//host/path` (protocol-relative — ambiguous scheme).
 *   - `/\host` (backslash-variant; some browsers normalise
 *     `\` → `/`).
 *   - bare relative (`about`, `./about`).
 *   - empty string, non-string.
 *
 * Error message includes the offending URL (truncated to 200
 * chars) so debugging is straightforward.
 */
export function resolveEntryUrl(url: unknown, normalisedBaseUrl: string): string {
  if (typeof url !== "string" || url.length === 0) {
    throw new TypeError(
      `forme-aot-sitemap-emitter: entry url must be a non-empty string; got ${
        url === null ? "null" : typeof url
      }`,
    );
  }
  if (isHttpUrl(url)) return url;
  if (isRootRelative(url)) {
    return url === "/" ? normalisedBaseUrl + "/" : normalisedBaseUrl + url;
  }
  const shown = url.length > 200 ? `${url.slice(0, 200)}…` : url;
  throw new TypeError(
    `forme-aot-sitemap-emitter: entry url must be http(s):// or root-relative /path; got ${
      JSON.stringify(shown)
    }`,
  );
}

/**
 * Case-insensitive `http://` / `https://` prefix check.  Only
 * lower-cases the first 8 chars to avoid a full-string
 * `toLowerCase()` allocation in the hot path.
 */
function isHttpUrl(url: string): boolean {
  const head = url.slice(0, 8).toLowerCase();
  return head.startsWith("http://") || head.startsWith("https://");
}

/**
 * Root-relative iff starts with exactly one `/` and NOT `//`
 * or `/\`.  Same `/\backslash-variant` defence as the
 * internal-links package.
 */
function isRootRelative(url: string): boolean {
  if (url === "/") return true;
  if (url.length < 2 || url[0] !== "/") return false;
  return url[1] !== "/" && url[1] !== "\\";
}
