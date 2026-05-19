/**
 * @coding-adventures/forme-transform-internal-links
 *
 * FM00 v0 §5.3 transform — resolve root-relative `/slug`
 * references in `LinkNode` destinations to caller-supplied
 * canonical URLs.
 *
 * Pure transform: walks the input document, calls
 * `(slug) => string | null` on every internal link, validates
 * the resolved URL against an http(s) accept-list, returns a
 * transformed `DocumentNode` copy.
 *
 * ```ts
 * import { rewriteInternalLinks } from "@coding-adventures/forme-transform-internal-links";
 *
 * function resolve(slug: string): string | null {
 *   const entry = manifest.byPath.get(slug);
 *   return entry ? entry.canonicalUrl : null;
 * }
 *
 * const linked = rewriteInternalLinks(doc, resolve);
 *
 * // Stricter: treat unresolved links as content bugs.
 * const validated = rewriteInternalLinks(doc, resolve, { unresolved: "throw" });
 * ```
 *
 * Eighth FM00 v0 stage package — joins `forme-feeds`,
 * `forme-opengraph`, `forme-index-renderer`, `forme-transforms`,
 * `forme-transform-autolink-headings`, `forme-transform-toc`,
 * `forme-transform-typography`.
 *
 * @module index
 */

export { rewriteInternalLinks } from "./walk.js";
export { isInternalSlug, assertResolvedUrl } from "./url.js";
export type {
  SlugResolver,
  UnresolvedPolicy,
  InternalLinksOptions,
} from "./types.js";
