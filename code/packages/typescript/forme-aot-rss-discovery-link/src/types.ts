/**
 * types.ts — FeedDiscoveryLink and supporting types.
 *
 * The `<link rel="alternate" type="application/rss+xml">` tag
 * is the de-facto convention for feed auto-discovery in
 * browsers and feed readers since the early 2000s.  Tag shape:
 *
 *   <link rel="alternate" type="<MIME>" href="<URL>" title="<TITLE>">
 *
 * `rel="alternate"` is fixed.  `type`, `href`, and `title` are
 * caller-supplied with validation / escaping.
 *
 * @module types
 */

/**
 * Allowed MIME types for the `type` attribute.  The three
 * mainstream feed formats:
 *
 *   - `application/rss+xml` — RSS 2.0 (default)
 *   - `application/atom+xml` — Atom 1.0
 *   - `application/json` — JSON Feed v1
 *
 * Anything else throws.  We intentionally don't accept
 * `application/rdf+xml` (RSS 1.0) since it's effectively
 * deprecated and accepting it would broaden the surface for
 * no benefit.
 */
export type FeedType =
  | "application/rss+xml"
  | "application/atom+xml"
  | "application/json";

/**
 * One feed discovery link entry.
 *
 *   - `href` (required) — feed URL (http(s):// OR
 *     root-relative `/path`).
 *   - `type` (optional) — MIME type from allowlist.  Defaults
 *     to `"application/rss+xml"` when omitted.
 *   - `title` (optional) — human-readable feed name shown by
 *     some feed readers in the discovery dropdown.
 *     HTML-attribute-escaped on emit.
 */
export interface FeedDiscoveryLink {
  readonly href: string;
  readonly type?: FeedType | string;
  readonly title?: string;
}
