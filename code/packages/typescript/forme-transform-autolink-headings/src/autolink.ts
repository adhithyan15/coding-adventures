/**
 * autolink.ts — the top-level transform entry point.
 *
 * Walks a `DocumentNode` once, finds every `HeadingNode` in
 * document order, extracts plain-text content, slugifies, and
 * resolves collisions deterministically.  Returns an ordered
 * `HeadingSlug[]` for downstream consumers (renderers, TOC
 * extractors, deep-link validators).
 *
 * The input document is NEVER mutated — the AST is immutable by
 * contract, and this transform is pure.
 *
 * Walk strategy: depth-first, document-order traversal of every
 * block whose body can contain other blocks (blockquote, list,
 * list_item, task_item, table cell).  Headings nested inside
 * blockquotes are slugified just like top-level ones; some
 * Markdown flavours allow this and Forme renderers must too.
 *
 * @module autolink
 */

import type {
  BlockNode,
  DocumentNode,
  HeadingNode,
} from "@coding-adventures/document-ast";
import { extractText } from "./extract-text.js";
import { slugify } from "./slugify.js";
import { resolveCollisions } from "./collisions.js";
import type { HeadingSlug } from "./types.js";

/**
 * Generate slug annotations for every heading in `doc`.
 *
 * Returned array is in document order — i.e. the same order a
 * pre-order DFS over `doc` would encounter the headings.
 *
 * ```ts
 * const slugs = autolinkHeadings(doc);
 * // slugs[0] is the first heading in the document, regardless of
 * // its level or nesting depth.
 * ```
 *
 * Reproducibility: same `DocumentNode` → byte-identical
 * `HeadingSlug[]`.
 */
export function autolinkHeadings(doc: DocumentNode): HeadingSlug[] {
  const headings: HeadingNode[] = [];
  collectHeadings(doc.children, headings);

  // Stage 1: compute the "base" slug for each heading from its text.
  const candidates: string[] = new Array(headings.length);
  const texts: string[] = new Array(headings.length);
  for (let i = 0; i < headings.length; i++) {
    const text = extractText(headings[i]!.children);
    texts[i] = text;
    candidates[i] = slugify(text);
  }

  // Stage 2: resolve collisions globally across the whole document.
  const resolved = resolveCollisions(candidates);

  // Stage 3: pack into HeadingSlug annotations.
  const out: HeadingSlug[] = new Array(headings.length);
  for (let i = 0; i < headings.length; i++) {
    const slug = resolved[i]!;
    out[i] = {
      level: headings[i]!.level,
      text: texts[i]!,
      slug,
      anchorHref: `#${slug}`,
    };
  }
  return out;
}

/**
 * Depth-first walk that pushes every `HeadingNode` found in
 * `blocks` into `acc` in document order.  Recurses into every
 * container-block kind defined by document-ast.
 */
function collectHeadings(blocks: readonly BlockNode[], acc: HeadingNode[]): void {
  for (let i = 0; i < blocks.length; i++) {
    const b = blocks[i]!;
    switch (b.type) {
      case "heading":
        acc.push(b);
        break;
      case "blockquote":
        collectHeadings(b.children, acc);
        break;
      case "list":
        // ListNode.children is ListItemNode | TaskItemNode.
        // Each item.children is BlockNode[].
        for (let j = 0; j < b.children.length; j++) {
          collectHeadings(b.children[j]!.children, acc);
        }
        break;
      case "table":
        // TableCellNode.children is InlineNode[] — no nested
        // blocks, so no headings to find.  Nothing to walk.
        break;
      case "paragraph":
      case "code_block":
      case "thematic_break":
      case "raw_block":
        // No nested blocks of interest.
        break;
      case "document":
      case "list_item":
      case "task_item":
      case "table_row":
      case "table_cell":
        // These BlockNode variants are not direct children of
        // other blocks in well-formed AST (DocumentNode is
        // root-only; list_item/task_item only appear inside
        // ListNode; table_row/table_cell only inside TableNode).
        // The walker reaches their content through the parent
        // cases above (list / table).  Defensive no-op for
        // type completeness.
        break;
      default: {
        const _exhaustive: never = b;
        void _exhaustive;
      }
    }
  }
}
