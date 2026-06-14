/**
 * types.ts — public signatures for the heading-anchors transform.
 *
 * @module types
 */

import type { DocumentNode, HeadingNode } from "@coding-adventures/document-ast";

/**
 * A `HeadingNode` augmented with a stable, URL-safe slug id.
 *
 * Structurally a `HeadingNode` (same `type`, `level`, `children`) plus a
 * single `id` field.  Downstream renderers cast (or duck-type) the
 * heading children of the returned document to this type to read the id
 * out — `id` is always present on every heading in the returned tree.
 */
export interface AnchoredHeadingNode extends HeadingNode {
  readonly id: string;
}

/**
 * One entry in the flat anchors list — useful for TOC builders and
 * cross-reference resolvers that don't want to re-walk the AST.
 */
export interface HeadingAnchor {
  /** The plain-text content of the heading, as fed to the slug algorithm. */
  readonly text: string;
  /** The slug id assigned to the heading (after collision suffixing). */
  readonly id: string;
  /** Heading depth, 1-6. */
  readonly level: 1 | 2 | 3 | 4 | 5 | 6;
  /** Reference to the (new) heading node in the returned document tree. */
  readonly heading: AnchoredHeadingNode;
}

/**
 * Result of `generateHeadingAnchors`.
 *
 * `document` is a NEW `DocumentNode` — non-heading children are
 * shared by reference with the input (every block / inline node in
 * document-ast is `readonly`, so sharing is safe).  Heading children
 * are replaced with `AnchoredHeadingNode` copies.
 *
 * `anchors` lists headings in document order — exactly what a TOC
 * builder wants without re-traversing.
 */
export interface HeadingAnchorsResult {
  readonly document: DocumentNode;
  readonly anchors: readonly HeadingAnchor[];
}
