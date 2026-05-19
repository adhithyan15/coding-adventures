/**
 * types.ts — sitemap entry and option types.
 *
 * Mirrors the sitemap.xml protocol per https://www.sitemaps.org/protocol.html
 * §4 (Optional and required tags).  The required tag is `<loc>`
 * — everything else is optional.
 *
 * @module types
 */

/**
 * Allowed values for the `<changefreq>` element per
 * https://www.sitemaps.org/protocol.html §4.
 *
 * Search engines treat this as a hint — they may or may not
 * crawl with the suggested frequency.  Validated via allowlist
 * before emission; anything else throws `TypeError`.
 */
export type ChangeFreq =
  | "always"
  | "hourly"
  | "daily"
  | "weekly"
  | "monthly"
  | "yearly"
  | "never";

/**
 * One entry in the sitemap.
 *
 *   - `url` (required) — http(s):// absolute OR root-relative
 *     `/path`.  Root-relative URLs are joined with `baseUrl`
 *     supplied to `generateSitemap`.  All other forms throw
 *     `TypeError` synchronously before any output is emitted.
 *   - `lastmod` (optional) — ISO-8601 date or date-time
 *     (`"2026-05-19"` or `"2026-05-19T14:00:00Z"`).  Passed
 *     through verbatim after XML-escaping; the spec doesn't
 *     require us to validate the format.
 *   - `changefreq` (optional) — see `ChangeFreq`.  Validated
 *     against allowlist.
 *   - `priority` (optional) — number in `[0.0, 1.0]`.  Values
 *     outside the range are clamped.  Emitted with one decimal
 *     place for byte-deterministic output.
 */
export interface SitemapEntry {
  readonly url: string;
  readonly lastmod?: string;
  readonly changefreq?: ChangeFreq | string;  // string for ergonomic input; validated downstream
  readonly priority?: number;
}
