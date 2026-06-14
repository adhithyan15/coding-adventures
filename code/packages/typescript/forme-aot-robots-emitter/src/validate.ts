/**
 * validate.ts — field-level validators for robots.txt values.
 *
 * The robots.txt protocol is line-oriented; any unescaped
 * newline or carriage return in a `Value` would split into
 * extra (possibly attacker-controlled) directive lines.  This
 * is the analogue of HTTP header injection for the line-based
 * robots format.  We REJECT (not strip) such inputs so the
 * caller knows about the bad data instead of silently shipping
 * a half-formed directive.
 *
 * Other validators here:
 *   - `validateHost` — DNS hostname-ish (no scheme, no path).
 *   - `validateSitemapUrl` — http(s)://, same accept-list as
 *     `forme-aot-sitemap-emitter`.
 *   - `validateCrawlDelay` — non-negative finite integer.
 *
 * @module validate
 */

/**
 * Forbidden control bytes in a robots.txt directive value:
 * CR, LF, NUL, and U+007F.  Any of these in user-supplied
 * input would either break the line-oriented parser or — for
 * CR/LF — inject a new directive line.
 *
 * Used as a guard, not a stripper: a single bad byte is enough
 * to throw.
 */
function hasInjectionChars(s: string): boolean {
  for (let i = 0; i < s.length; i++) {
    const c = s.charCodeAt(i);
    if (c === 0x0A || c === 0x0D || c === 0x00 || c === 0x7F) return true;
    // Also reject the rest of the C0 controls — none have a
    // legitimate place in robots.txt and accepting them just
    // gives attackers a wider surface.
    if (c < 0x20 && c !== 0x09) return true;  // allow TAB; reject other C0
  }
  return false;
}

/**
 * Validate a single User-agent / Allow / Disallow value.
 * Throws `TypeError` if the value contains injection
 * characters or isn't a non-empty string.
 *
 * Returns the input unchanged on success — the caller emits it
 * verbatim into a line.
 */
export function validateDirectiveValue(value: string, field: string): string {
  if (typeof value !== "string" || value.length === 0) {
    throw new TypeError(
      `forme-aot-robots-emitter: ${field} must be a non-empty string; got ${
        value === null ? "null" : typeof value
      }`,
    );
  }
  if (hasInjectionChars(value)) {
    throw new TypeError(
      `forme-aot-robots-emitter: ${field} contains forbidden control character (CR/LF/NUL/DEL or other C0); refusing to emit (injection risk)`,
    );
  }
  return value;
}

/**
 * Validate `crawlDelay` per the de-facto convention used by
 * Bing, Yandex, etc.: non-negative finite integer (seconds).
 * Zero is permitted (means "no throttling").
 *
 * Throws `TypeError` if not a number, NaN, infinite, negative,
 * or non-integer.
 *
 * Returns the integer as-is.
 */
export function validateCrawlDelay(value: number): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new TypeError(
      `forme-aot-robots-emitter: crawlDelay must be a finite number; got ${typeof value === "number" ? value : typeof value}`,
    );
  }
  if (!Number.isInteger(value)) {
    throw new TypeError(
      `forme-aot-robots-emitter: crawlDelay must be an integer; got ${value}`,
    );
  }
  if (value < 0) {
    throw new TypeError(
      `forme-aot-robots-emitter: crawlDelay must be non-negative; got ${value}`,
    );
  }
  return value;
}

/**
 * Validate a sitemap URL.  Same accept-list as
 * `forme-aot-sitemap-emitter`'s `assertResolvedUrl`:
 * `http(s)://...` only (case-insensitive scheme).  Anything
 * else throws.
 *
 * Why no root-relative support?  The `Sitemap:` directive in
 * robots.txt is documented as taking an absolute URL.
 * Root-relative paths are ambiguous (relative to which host?)
 * and rejected by most crawlers.
 */
export function validateSitemapUrl(url: string): string {
  if (typeof url !== "string" || url.length === 0) {
    throw new TypeError(
      `forme-aot-robots-emitter: sitemap URL must be a non-empty string; got ${
        url === null ? "null" : typeof url
      }`,
    );
  }
  if (hasInjectionChars(url)) {
    throw new TypeError(
      `forme-aot-robots-emitter: sitemap URL contains forbidden control character; refusing to emit`,
    );
  }
  const head = url.slice(0, 8).toLowerCase();
  if (!(head.startsWith("http://") || head.startsWith("https://"))) {
    const shown = url.length > 200 ? url.slice(0, 200) + "…" : url;
    throw new TypeError(
      `forme-aot-robots-emitter: sitemap URL must be http(s)://; got ${JSON.stringify(shown)}`,
    );
  }
  return url;
}

/**
 * Validate a `host:` value.  Per Yandex's documentation this
 * is a host (and optional port), NOT a URL.  We accept
 * anything that:
 *
 *   - is a non-empty string,
 *   - has no injection characters,
 *   - has no scheme (rejected: anything containing `://`),
 *   - has no path / query / fragment (rejected: `/`, `?`, `#`).
 *
 * The character set isn't strictly DNS-validated (RFC 1123)
 * because the directive is widely-ignored anyway — we just
 * guard against the obvious injection / scheme-confusion
 * attacks.
 */
export function validateHost(host: string): string {
  if (typeof host !== "string" || host.length === 0) {
    throw new TypeError(
      `forme-aot-robots-emitter: host must be a non-empty string; got ${
        host === null ? "null" : typeof host
      }`,
    );
  }
  if (hasInjectionChars(host)) {
    throw new TypeError(
      `forme-aot-robots-emitter: host contains forbidden control character; refusing to emit`,
    );
  }
  if (host.includes("://")) {
    throw new TypeError(
      `forme-aot-robots-emitter: host must be a bare hostname, not a URL; got ${JSON.stringify(host)}`,
    );
  }
  if (host.includes("/") || host.includes("?") || host.includes("#") || host.includes(" ")) {
    throw new TypeError(
      `forme-aot-robots-emitter: host must be a bare hostname (no path / query / fragment / spaces); got ${JSON.stringify(host)}`,
    );
  }
  return host;
}
