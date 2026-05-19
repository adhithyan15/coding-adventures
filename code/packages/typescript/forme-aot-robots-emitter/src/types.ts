/**
 * types.ts — RobotsConfig and rule shapes.
 *
 * The robots.txt protocol is line-oriented: a sequence of
 * `Key: Value` pairs grouped into rules.  This type set is the
 * structured JS shape that maps onto that line format.
 *
 * @module types
 */

/**
 * One robots.txt rule block (`User-agent:` + zero or more
 * `Allow:` / `Disallow:` lines, plus optional `Crawl-delay:`).
 *
 *   - `userAgent` (required) — either a single string
 *     (`"Googlebot"`) or an array (one `User-agent:` line per
 *     element).  `"*"` matches all crawlers.
 *   - `allow` (optional) — paths that ARE permitted.  Each path
 *     emitted as its own `Allow:` line.
 *   - `disallow` (optional) — paths that are NOT permitted.
 *     Each path emitted as its own `Disallow:` line.
 *   - `crawlDelay` (optional) — seconds between requests.  Must
 *     be a non-negative finite integer; non-integers throw.
 *     Google ignores this directive; most other crawlers respect
 *     it.  Emitted as `Crawl-delay: N`.
 */
export interface RobotsRule {
  readonly userAgent: string | readonly string[];
  readonly allow?: readonly string[];
  readonly disallow?: readonly string[];
  readonly crawlDelay?: number;
}

/**
 * Top-level robots.txt configuration.
 *
 *   - `rules` (required, may be empty) — ordered rule blocks.
 *   - `sitemap` (optional) — one or more sitemap URLs.  Each
 *     URL emitted as a `Sitemap:` line.  Validated against
 *     http(s):// before emission.
 *   - `host` (optional) — preferred host directive (Yandex
 *     extension, widely-ignored but harmless).  Validated as
 *     a hostname (no scheme; no path).
 *
 * Emit order (matches widely-recognised convention):
 *   1. Rule blocks in input order, separated by blank lines.
 *   2. `Sitemap:` lines (one per URL).
 *   3. `Host:` line (if supplied).
 */
export interface RobotsConfig {
  readonly rules: readonly RobotsRule[];
  readonly sitemap?: string | readonly string[];
  readonly host?: string;
}
