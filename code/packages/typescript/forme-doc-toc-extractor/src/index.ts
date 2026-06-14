/**
 * @coding-adventures/forme-doc-toc-extractor
 *
 * Walk a `DocumentNode` AST and produce a nested table-of-contents
 * tree — the outline a sidebar widget, in-page TOC, or PDF
 * bookmarks pane renders.
 *
 * The TOC is built by nesting headings by depth using a classic
 * stack-based algorithm: each heading of level N becomes a child of
 * the most recent heading of level < N.  Skipped levels (h1 → h3 with
 * no h2) collapse cleanly — h3 nests directly under h1.  Multiple h1s
 * become multiple top-level entries (no auto-grouping under a
 * synthetic root).
 *
 * Slugification is delegated to
 * `@coding-adventures/forme-doc-heading-anchors` — every heading in
 * the output tree carries the same `id` as the corresponding
 * heading in the returned anchored AST, so HTML renderers can use
 * the AST's `heading.id` while sidebar code uses the tree's `id`
 * and both stay in sync.
 *
 * Pure transform.  Capabilities: `[]`.  No `eval`, `new Function`,
 * `JSON.parse` reviver, fs, network, env, or shell.  Both transitive
 * deps (`forme-doc-heading-anchors`, `document-ast`) are also
 * `[]`-capability.
 *
 * ```ts
 * import { extractToc } from "@coding-adventures/forme-doc-toc-extractor";
 * import { parseCommonMark } from "@coding-adventures/commonmark-parser";
 *
 * const doc = parseCommonMark(`
 * # Introduction
 * ## Setup
 * ### Prerequisites
 * ## Quick start
 * # Reference
 * ## API
 * `);
 *
 * const { toc, document, anchors } = extractToc(doc);
 * // toc = [
 * //   { text: "Introduction", id: "introduction", level: 1, children: [
 * //     { text: "Setup",       id: "setup",        level: 2, children: [
 * //       { text: "Prerequisites", id: "prerequisites", level: 3, children: [] },
 * //     ]},
 * //     { text: "Quick start", id: "quick-start",  level: 2, children: [] },
 * //   ]},
 * //   { text: "Reference",    id: "reference",    level: 1, children: [
 * //     { text: "API",          id: "api",          level: 2, children: [] },
 * //   ]},
 * // ]
 * ```
 *
 * Third concrete DOC00 v0 package (after `forme-doc-frontmatter` and
 * `forme-doc-heading-anchors`).
 *
 * @module index
 */

export { extractToc, buildTocTree } from "./extractor.js";
export type { TocEntry, TocResult } from "./types.js";
