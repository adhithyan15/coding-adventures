/**
 * @coding-adventures/forme-collect-by-tag
 *
 * FM00 v0 §5.4 collector — group documents by tag.
 *
 * Pure transform: items array + `tagsOf` accessor → Map keyed
 * by normalised tag bucket → items sorted within each bucket.
 *
 * ```ts
 * import { collectByTag } from "@coding-adventures/forme-collect-by-tag";
 *
 * const { byTag, tagNames } = collectByTag(posts, {
 *   tagsOf: (p) => p.tags,
 *   sortBy: (a, b) => b.pubDate.localeCompare(a.pubDate),  // newest first
 *   includeUntagged: true,
 * });
 *
 * for (const tag of tagNames) {
 *   const archivePage = renderTagArchive(tag, byTag.get(tag)!);
 * }
 * ```
 *
 * Sits alongside `forme-collect-chronological` as the second
 * concrete §5.4 collector.  Joins the FM00 v0 stage cluster:
 * `forme-feeds`, `forme-opengraph`, `forme-index-renderer`,
 * `forme-transforms`, `forme-transform-autolink-headings`,
 * `forme-transform-toc`, `forme-transform-typography`,
 * `forme-transform-internal-links`.
 *
 * @module index
 */

export { collectByTag } from "./collect.js";
export { normaliseTag } from "./normalise.js";
export type {
  CollectByTagOptions,
  CollectByTagResult,
  TagsOf,
} from "./types.js";
