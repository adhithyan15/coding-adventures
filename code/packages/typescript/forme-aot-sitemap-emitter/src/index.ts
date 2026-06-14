/**
 * @coding-adventures/forme-aot-sitemap-emitter
 *
 * Emit `sitemap.xml` from a `SitemapEntry[]` + `baseUrl` per
 * https://www.sitemaps.org/protocol.html.  Pure transform —
 * returns the XML string; caller decides where to write it.
 *
 * ```ts
 * import { generateSitemap } from "@coding-adventures/forme-aot-sitemap-emitter";
 *
 * const xml = generateSitemap([
 *   { url: "/",              lastmod: "2026-05-19", changefreq: "daily",   priority: 1.0 },
 *   { url: "/about",         lastmod: "2026-05-15", changefreq: "monthly", priority: 0.8 },
 *   { url: "https://other.example/x"                                                       },
 * ], "https://example.com");
 *
 * fs.writeFileSync("dist/sitemap.xml", xml);
 * ```
 *
 * Validation runs BEFORE emission — bad URL schemes, bad
 * changefreq values, bad baseUrl all throw `TypeError`
 * synchronously and no partial output reaches the caller.
 *
 * Eleventh FM00 v0 stage package — joins the FM00 v0 cluster.
 *
 * @module index
 */

export { generateSitemap } from "./generate.js";
export { escapeXml, stripInvalidXml } from "./escape.js";
export { normaliseBaseUrl, resolveEntryUrl } from "./url.js";
export { validateChangefreq, clampPriority } from "./validate.js";
export type { ChangeFreq, SitemapEntry } from "./types.js";
