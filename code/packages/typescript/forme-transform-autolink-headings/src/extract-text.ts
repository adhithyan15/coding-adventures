/**
 * extract-text.ts — flatten a heading's inline children to plain text.
 *
 * Headings can contain rich inline content — text, emphasis,
 * strong, code spans, links, images, raw HTML.  Slug generation
 * (and TOC labels) need a plain-text view.  This walk concatenates
 * the textual content of every inline child, recursing into
 * formatting wrappers.
 *
 * Per-node behaviour:
 *
 * | Inline node       | Text contribution                          |
 * |-------------------|--------------------------------------------|
 * | `text`            | `value` verbatim                           |
 * | `emphasis`        | recurse into children                      |
 * | `strong`          | recurse into children                      |
 * | `strikethrough`   | recurse into children                      |
 * | `link`            | recurse into children (link label)         |
 * | `code_span`       | `value` verbatim (preserves source code)   |
 * | `image`           | `alt` text (matches screen-reader output)  |
 * | `autolink`        | `destination` (URL or email address)       |
 * | `raw_inline`      | skipped (back-end-specific markup)         |
 * | `hard_break`      | single space                               |
 * | `soft_break`      | single space                               |
 *
 * `raw_inline` is intentionally skipped — its content is back-end-
 * specific (e.g. `<sup>x</sup>` for HTML) and including it would
 * leak markup into slugs (`stepx` vs `step`) and TOC labels.
 *
 * @module extract-text
 */

import type { InlineNode } from "@coding-adventures/document-ast";

/**
 * Concatenate the plain-text content of an inline node list.
 * Returns a single string with leading / trailing whitespace
 * trimmed and internal whitespace collapsed via `slugify` later
 * (this function preserves spacing so TOC labels read naturally).
 */
export function extractText(nodes: readonly InlineNode[]): string {
  const parts: string[] = [];
  walk(nodes, parts);
  return parts.join("");
}

function walk(nodes: readonly InlineNode[], out: string[]): void {
  for (let i = 0; i < nodes.length; i++) {
    const n = nodes[i]!;
    switch (n.type) {
      case "text":
        out.push(n.value);
        break;
      case "emphasis":
      case "strong":
      case "strikethrough":
      case "link":
        walk(n.children, out);
        break;
      case "code_span":
        out.push(n.value);
        break;
      case "image":
        out.push(n.alt);
        break;
      case "autolink":
        out.push(n.destination);
        break;
      case "hard_break":
      case "soft_break":
        out.push(" ");
        break;
      case "raw_inline":
        // Intentionally skipped — back-end-specific markup
        // shouldn't leak into slugs / TOC labels.
        break;
      default: {
        // Exhaustiveness guard.  If a new InlineNode kind is
        // added to document-ast, TypeScript flags this branch.
        const _exhaustive: never = n;
        void _exhaustive;
      }
    }
  }
}
