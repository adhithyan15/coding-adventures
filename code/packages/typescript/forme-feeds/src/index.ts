/**
 * @coding-adventures/forme-feeds
 *
 * RSS 2.0 + Atom 1.0 feed generators for the Forme pipeline (FM00 v0).
 *
 * Pure transform — `ChannelMeta | FeedMeta + items[]` → reproducible
 * XML string.  Same inputs always produce byte-identical output; no
 * timestamps injected, no random IDs, no I/O.
 *
 * ```ts
 * import { generateRssFeed, generateAtomFeed } from "@coding-adventures/forme-feeds";
 *
 * const rssXml = generateRssFeed(
 *   { title: "My Blog", link: "https://example.com/", description: "..." },
 *   posts.map((p) => ({ id: p.url, title: p.title, link: p.url, content: p.excerpt })),
 * );
 *
 * const atomXml = generateAtomFeed(
 *   { id: "https://example.com/atom.xml", title: "My Blog", updated: "2026-05-17T00:00:00Z" },
 *   posts.map((p) => ({ id: p.url, title: p.title, link: p.url, contentHtml: p.html })),
 * );
 * ```
 *
 * @module index
 */

export { generateRssFeed } from "./rss.js";
export { generateAtomFeed } from "./atom.js";
export { escapeXml, stripInvalidXml, wrapCdata } from "./escape.js";
export type { ChannelMeta, FeedMeta, FeedItem } from "./types.js";
