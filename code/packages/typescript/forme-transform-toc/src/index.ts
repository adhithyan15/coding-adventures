/**
 * @coding-adventures/forme-transform-toc
 *
 * FM00 v0 §5.3 transform — build a hierarchical Table-of-Contents
 * tree from a `DocumentNode` (or a pre-computed `HeadingSlug[]`).
 *
 * Pure transform: heading sequence → nested `TocNode[]`
 * (`{ level, text, slug, href, children: TocNode[] }`).
 * Renderers walk the tree depth-first to emit
 *
 *   <nav class="forme-toc">
 *     <ul>
 *       <li>
 *         <a href="#installation">Installation</a>
 *         <ul>
 *           <li><a href="#requirements">Requirements</a></li>
 *         </ul>
 *       </li>
 *     </ul>
 *   </nav>
 *
 * ```ts
 * import { buildToc } from "@coding-adventures/forme-transform-toc";
 *
 * const toc = buildToc(doc, { minLevel: 2, maxLevel: 4 });
 * ```
 *
 * Built on top of `forme-transform-autolink-headings` for the
 * slug stream.  Sixth FM00 v0 stage package — joins
 * `forme-feeds`, `forme-opengraph`, `forme-index-renderer`,
 * `forme-transforms`, `forme-transform-autolink-headings`.
 *
 * @module index
 */

export { buildToc, buildTocFromSlugs } from "./build-toc.js";
export { buildTree } from "./build-tree.js";
export { filterByLevel } from "./filter.js";
export type { TocNode, TocOptions } from "./types.js";
