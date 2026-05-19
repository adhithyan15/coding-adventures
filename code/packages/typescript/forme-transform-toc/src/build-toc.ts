/**
 * build-toc.ts — top-level entry points.
 *
 * Two flavours of the same transform, depending on what the
 * caller already has:
 *
 *   - `buildToc(doc, options?)` — caller has a fresh
 *     `DocumentNode`; we walk it via `autolinkHeadings` to get
 *     the flat slug stream, then build the tree.  Use this in
 *     simple pipelines.
 *   - `buildTocFromSlugs(slugs, options?)` — caller already
 *     called `autolinkHeadings` for some other reason (e.g. to
 *     pass slugs to the renderer) and doesn't want to repeat
 *     the walk.  Cheaper.
 *
 * Both go through the same `filterByLevel` → `buildTree`
 * pipeline, so behaviour is identical given the same effective
 * slug stream.
 *
 * @module build-toc
 */

import type { DocumentNode } from "@coding-adventures/document-ast";
import {
  autolinkHeadings,
  type HeadingSlug,
} from "@coding-adventures/forme-transform-autolink-headings";
import { filterByLevel } from "./filter.js";
import { buildTree } from "./build-tree.js";
import type { TocNode, TocOptions } from "./types.js";

/**
 * Build a TOC tree directly from a `DocumentNode`.
 *
 * Internally runs `autolinkHeadings(doc)` to get the slug
 * stream.  Callers who already have the slugs should use
 * `buildTocFromSlugs` to avoid the duplicate walk.
 */
export function buildToc(doc: DocumentNode, options: TocOptions = {}): TocNode[] {
  return buildTocFromSlugs(autolinkHeadings(doc), options);
}

/**
 * Build a TOC tree from a pre-computed `HeadingSlug[]`.
 *
 * Useful when the caller is already running `autolinkHeadings`
 * for other purposes (renderer consumes the slug stream
 * directly to emit `<h2 id="…">` markup) and doesn't want to
 * walk the AST twice.
 */
export function buildTocFromSlugs(
  slugs: readonly HeadingSlug[],
  options: TocOptions = {},
): TocNode[] {
  const minLevel = options.minLevel ?? 1;
  const maxLevel = options.maxLevel ?? 6;
  const filtered = filterByLevel(slugs, minLevel, maxLevel);
  return buildTree(filtered);
}
