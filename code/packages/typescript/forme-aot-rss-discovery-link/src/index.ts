/**
 * @coding-adventures/forme-aot-rss-discovery-link
 *
 * Emit HTML `<link rel="alternate" type="application/rss+xml">`
 * tags for feed auto-discovery.  Pure transform — returns the
 * tag string; caller drops it into their HTML `<head>`.
 *
 * ```ts
 * import { generateFeedDiscoveryLinks } from "@coding-adventures/forme-aot-rss-discovery-link";
 *
 * // Single feed:
 * const tag = generateFeedDiscoveryLinks({
 *   href: "/feed.xml",
 *   title: "My Blog",
 * });
 * // <link rel="alternate" type="application/rss+xml" title="My Blog" href="/feed.xml">
 *
 * // Multiple feeds (RSS + Atom + JSON Feed):
 * const tags = generateFeedDiscoveryLinks([
 *   { href: "/feed.xml",  type: "application/rss+xml",  title: "RSS" },
 *   { href: "/atom.xml",  type: "application/atom+xml", title: "Atom" },
 *   { href: "/feed.json", type: "application/json",     title: "JSON Feed" },
 * ]);
 * ```
 *
 * Validation runs BEFORE emission — bad URLs, bad types, bad
 * titles all throw `TypeError` synchronously.
 *
 * Fourteenth FM00 v0 stage package — joins the FM00 v0 cluster.
 *
 * @module index
 */

export { generateFeedDiscoveryLinks } from "./generate.js";
export { escapeHtmlAttr, stripAsciiControl } from "./escape.js";
export { validateFeedHref, validateFeedType } from "./validate.js";
export type { FeedDiscoveryLink, FeedType } from "./types.js";
