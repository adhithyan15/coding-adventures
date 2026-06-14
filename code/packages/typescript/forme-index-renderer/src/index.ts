/**
 * @coding-adventures/forme-index-renderer
 *
 * Index / archive page renderer for the Forme pipeline (FM00 v0).
 *
 * Pure transform: `IndexItem[]` + `IndexOptions` → reproducible
 * HTML `<ul>` (optionally grouped) suitable for blog archives.
 * Pairs with `forme-aot-page-emitter` (the emitter writes the
 * `.html` wrapper, this renderer fills the body).
 *
 * ```ts
 * import { renderIndexPage } from "@coding-adventures/forme-index-renderer";
 *
 * const html = renderIndexPage(items, {
 *   groupBy: "year",
 *   sortBy: "pubDate-desc",
 *   showDate: true,
 *   showSummary: true,
 *   dateFormat: (iso) => new Date(iso).toLocaleDateString("en-US"),
 * });
 * ```
 *
 * @module index
 */

export { renderIndexPage } from "./index-renderer.js";
export { groupItems } from "./group.js";
export { sortItems } from "./sort.js";
export { escapeHtmlAttr, escapeHtmlText, assertItemUrl } from "./escape.js";
export type { IndexItem, IndexOptions, ItemGroup } from "./types.js";
