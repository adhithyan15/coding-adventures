/**
 * types.ts — public signatures for the TOC extractor.
 *
 * @module types
 */

import type { DocumentNode } from "@coding-adventures/document-ast";
import type { HeadingAnchor } from "@coding-adventures/forme-doc-heading-anchors";

/**
 * One node of a hierarchical table of contents.
 *
 * `text`, `id`, and `level` mirror the underlying heading; `children`
 * holds entries whose heading depth was strictly greater than this
 * entry's `level` and appeared after it in the source order, up to
 * (but not including) the next sibling or shallower heading.
 *
 * The shape is intentionally JSON-able — no AST node references, no
 * symbols — so sidebar widgets, in-page TOC scripts, and
 * `JSON.stringify`-based caches can consume it directly.
 */
export interface TocEntry {
  /** Plain-text heading content (markup elided). */
  readonly text: string;
  /** URL-safe slug — matches the heading's `id` in the anchored AST. */
  readonly id: string;
  /** Original heading depth, 1-6. */
  readonly level: 1 | 2 | 3 | 4 | 5 | 6;
  /** Nested entries — headings of greater depth that follow this one. */
  readonly children: readonly TocEntry[];
}

/**
 * Result of `extractToc`.
 *
 * `toc` is the hierarchical tree.  `document` is the anchored
 * DocumentNode (with `AnchoredHeadingNode` heading children — same as
 * `generateHeadingAnchors` would return), so downstream HTML renderers
 * can use `heading.id` directly without re-walking.  `anchors` is the
 * flat in-document-order list — useful for any consumer that wants
 * sequential iteration instead of recursion.
 *
 * The three projections (tree, anchored AST, flat list) are derived
 * from the same source-order traversal and are guaranteed consistent
 * — same headings, same slugs, same collision suffixes.
 */
export interface TocResult {
  readonly toc: readonly TocEntry[];
  readonly document: DocumentNode;
  readonly anchors: readonly HeadingAnchor[];
}
