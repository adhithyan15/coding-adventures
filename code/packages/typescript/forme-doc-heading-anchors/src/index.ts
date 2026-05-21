/**
 * @coding-adventures/forme-doc-heading-anchors
 *
 * Walk a `DocumentNode` AST, generate a URL-safe slug ID for every
 * heading, and return a new tree whose `HeadingNode`s carry an `id`
 * field — plus a parallel flat list of heading metadata for downstream
 * consumers (TOC builder, cross-reference resolver, sidebar widget).
 *
 * Slug derivation follows GitHub's well-known algorithm: lowercase
 * the heading text, strip anything that isn't a Unicode word
 * character / hyphen / space, replace spaces with hyphens.  Within a
 * single document, collisions get `-1` / `-2` / … suffixes (the first
 * occurrence keeps the bare slug — matching GitHub).
 *
 * Pure transform.  Capabilities: `[]`.  No `eval`, no `new Function`,
 * no `JSON.parse` reviver, no fs / network / env / shell.
 *
 * ```ts
 * import { generateHeadingAnchors } from "@coding-adventures/forme-doc-heading-anchors";
 * import { parseCommonMark } from "@coding-adventures/commonmark-parser";
 *
 * const doc = parseCommonMark("# Getting Started\n## API Reference");
 * const { document, anchors } = generateHeadingAnchors(doc);
 * // anchors[0] = { text: "Getting Started", id: "getting-started", level: 1, heading: <node> }
 * // anchors[1] = { text: "API Reference",   id: "api-reference",   level: 2, heading: <node> }
 * ```
 *
 * Second concrete DOC00 v0 package (after `forme-doc-frontmatter`).
 *
 * @module index
 */

export { generateHeadingAnchors } from "./walker.js";
export { slugify } from "./slug.js";
export type {
  AnchoredHeadingNode,
  HeadingAnchor,
  HeadingAnchorsResult,
} from "./types.js";
