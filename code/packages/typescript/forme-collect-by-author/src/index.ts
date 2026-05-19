/**
 * @coding-adventures/forme-collect-by-author
 *
 * FM00 v0 §5.4 collector — group documents by author.
 *
 * Pure transform: items array + `authorOf` accessor → Map
 * keyed by normalised author bucket → items sorted within each
 * bucket.  Mirrors `forme-collect-by-tag` but the accessor
 * accepts `string | string[]` so co-author posts cleanly land
 * in every contributor's bucket.
 *
 * ```ts
 * import { collectByAuthor } from "@coding-adventures/forme-collect-by-author";
 *
 * const { byAuthor, authorNames } = collectByAuthor(posts, {
 *   authorOf: (p) => p.author ?? p.authors,
 *   sortBy: (a, b) => b.pubDate.localeCompare(a.pubDate),
 *   includeAnonymous: true,
 * });
 *
 * for (const author of authorNames) {
 *   const archivePage = renderAuthorArchive(author, byAuthor.get(author)!);
 * }
 * ```
 *
 * Tenth FM00 v0 stage package — joins `forme-feeds`,
 * `forme-opengraph`, `forme-index-renderer`, `forme-transforms`,
 * `forme-transform-autolink-headings`, `forme-transform-toc`,
 * `forme-transform-typography`,
 * `forme-transform-internal-links`, `forme-collect-by-tag`.
 *
 * @module index
 */

export { collectByAuthor } from "./collect.js";
export { normaliseAuthor } from "./normalise.js";
export type {
  CollectByAuthorOptions,
  CollectByAuthorResult,
  AuthorOf,
} from "./types.js";
