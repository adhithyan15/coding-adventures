/**
 * build-tree.ts — flat `HeadingSlug[]` → hierarchical `TocNode[]`.
 *
 * The algorithm is a single forward pass over the flat heading
 * list, maintaining a stack of "currently-open" ancestor nodes.
 * For each incoming heading:
 *
 *   1. Pop ancestors whose level is >= the new heading's level
 *      (their subtree is closed).
 *   2. If the stack is empty after popping, the new node is a
 *      top-level root.
 *   3. Otherwise, the new node is appended as a child of the
 *      stack top.
 *   4. Push the new node onto the stack so deeper headings
 *      attach beneath it.
 *
 * Cost: O(N) — each heading is pushed once and popped at most
 * once.
 *
 * Malformed sequences (the spec calls this out as a v0
 * requirement):
 *
 *   - **Skipped levels** (`h1` directly to `h3`) — the `h3` just
 *     becomes a child of the `h1`.  No interpolated empty `h2`,
 *     no thrown error.  Most TOC renderers handle this fine
 *     (the nested `<ul>` just appears one level deeper).
 *   - **Outdent past root** — a `h1` after a deep nesting closes
 *     all open subtrees and starts a new root.  Standard stack
 *     pop semantics.
 *   - **Repeated peer levels** — three sibling `h2`s under one
 *     `h1` produce three children in the parent's `children`
 *     array, in input order.
 *   - **Document starts with a deep level** (e.g. first heading
 *     is `h3`) — that `h3` becomes a top-level root.  The TOC
 *     reflects what the document actually contains; it doesn't
 *     manufacture missing ancestors.
 *
 * Determinism: same input → byte-identical output.  The stack
 * algorithm is a pure function of the input order.
 *
 * @module build-tree
 */

import type { HeadingSlug } from "@coding-adventures/forme-transform-autolink-headings";
import type { TocNode } from "./types.js";

/**
 * Mutable scratch type used during construction.  Once the tree
 * is built, every node is frozen-ish (children is read as
 * `readonly TocNode[]` by the public type).  We don't `Object.freeze`
 * for performance; the `readonly` contract is the API guarantee.
 */
interface MutableTocNode {
  level: 1 | 2 | 3 | 4 | 5 | 6;
  text: string;
  slug: string;
  href: string;
  children: MutableTocNode[];
}

/**
 * Build the TOC tree from an ordered `HeadingSlug[]`.  Returns
 * the top-level roots.
 *
 * Roots are headings whose level is shallower than every
 * preceding heading at the point of insertion (i.e. they pop the
 * whole stack).  In a well-formed document with one `h1`, there
 * is exactly one root.  In a documentation site with multiple
 * top-level sections starting at `h2`, there can be multiple
 * roots.
 *
 * Input is never mutated.  Output `children` arrays are mutable
 * during construction but exposed as `readonly` via the public
 * type.
 */
export function buildTree(slugs: readonly HeadingSlug[]): TocNode[] {
  const roots: MutableTocNode[] = [];
  const stack: MutableTocNode[] = [];
  for (let i = 0; i < slugs.length; i++) {
    const s = slugs[i]!;
    const node: MutableTocNode = {
      level: s.level,
      text: s.text,
      slug: s.slug,
      href: s.anchorHref,
      children: [],
    };
    // Pop ancestors at the same or deeper level — they cannot be
    // this node's parent.
    while (stack.length > 0 && stack[stack.length - 1]!.level >= s.level) {
      stack.pop();
    }
    if (stack.length === 0) {
      roots.push(node);
    } else {
      stack[stack.length - 1]!.children.push(node);
    }
    stack.push(node);
  }
  return roots;
}
