/**
 * types.ts — TOC tree node shape and options.
 *
 * A `TocNode` is the hierarchical counterpart of the flat
 * `HeadingSlug` from `forme-transform-autolink-headings`: each
 * heading becomes one node, and `children` holds deeper-level
 * headings nested beneath it.
 *
 * Renderers consume the tree depth-first to emit
 *
 *   <nav class="forme-toc">
 *     <ul>
 *       <li>
 *         <a href="#installation">Installation</a>
 *         <ul>
 *           <li><a href="#requirements">Requirements</a></li>
 *           <li><a href="#install-steps">Install steps</a></li>
 *         </ul>
 *       </li>
 *     </ul>
 *   </nav>
 *
 * The `level` field is preserved so renderers can apply
 * level-specific classes if desired (e.g. `class="toc-h1"`).
 *
 * @module types
 */

/**
 * One node in the TOC tree.  Mirrors `HeadingSlug` plus a
 * `children` array for nested headings.
 *
 * Invariants enforced by `buildToc`:
 *   - `children[i].level > this.level` (children are always
 *     deeper).
 *   - `slug` is unique within the whole tree (collision-resolved
 *     upstream).
 *   - `href === "#" + slug`.
 */
export interface TocNode {
  readonly level: 1 | 2 | 3 | 4 | 5 | 6;
  readonly text: string;
  readonly slug: string;
  readonly href: string;
  readonly children: readonly TocNode[];
}

/**
 * Options controlling which headings appear in the output tree.
 *
 *   - `minLevel` (default `1`) — drop headings shallower than
 *     this.  E.g. `minLevel: 2` skips `<h1>` (the page title is
 *     usually outside the TOC).
 *   - `maxLevel` (default `6`) — drop headings deeper than this.
 *     E.g. `maxLevel: 3` keeps only `<h1>`/`<h2>`/`<h3>` in the
 *     TOC; `<h4>`+ get omitted entirely (and don't contribute
 *     nesting either).
 *
 * Out-of-range filtering happens BEFORE tree construction.  A
 * filtered-out heading does not interrupt the hierarchy of its
 * surviving neighbours.
 */
export interface TocOptions {
  readonly minLevel?: 1 | 2 | 3 | 4 | 5 | 6;
  readonly maxLevel?: 1 | 2 | 3 | 4 | 5 | 6;
}
