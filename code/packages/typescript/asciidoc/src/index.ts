/**
 * @coding-adventures/asciidoc
 *
 * AsciiDoc pipeline convenience package.
 *
 * Combines the AsciiDoc parser and the Document AST HTML renderer into a
 * simple two-function API:
 *
 *   - `parse(asciidoc)` — from @coding-adventures/asciidoc-parser
 *   - `render(doc)`     — from @coding-adventures/document-ast-to-html
 *
 * ```
 *   @coding-adventures/document-ast        ← format-agnostic types
 *          ↓ types                                ↓ types
 *   @coding-adventures/asciidoc-parser    @coding-adventures/document-ast-to-html
 *     parse(asciidoc) → DocumentNode         render(doc) → string
 *          ↓ depends on both
 *   @coding-adventures/asciidoc            ← you are here
 *     const html = toHtml(asciidoc)
 * ```
 *
 * === Quick Start ===
 *
 * ```typescript
 * import { toHtml } from "@coding-adventures/asciidoc";
 *
 * const html = toHtml("= Hello\n\nWorld *bold*.\n");
 * // → "<h1>Hello</h1>\n<p>World <strong>bold</strong>.</p>\n"
 * ```
 *
 * Users who need the AST or a different renderer should import the constituent
 * packages directly:
 *
 * ```typescript
 * import { parse } from "@coding-adventures/asciidoc-parser";
 * import { render } from "@coding-adventures/document-ast-to-html";
 * import type { DocumentNode } from "@coding-adventures/document-ast";
 *
 * const doc: DocumentNode = parse(asciidocString);
 * const html = render(doc);
 * ```
 *
 * @module index
 */

import { parse } from "@coding-adventures/asciidoc-parser";
import { toHtml as renderToHtml } from "@coding-adventures/document-ast-to-html";

// ─── Re-exports ───────────────────────────────────────────────────────────────

export { parse } from "@coding-adventures/asciidoc-parser";
export { toHtml as render } from "@coding-adventures/document-ast-to-html";

export type {
  // Node union types
  DocumentNode, BlockNode, InlineNode,
  // Block node types
  HeadingNode, ParagraphNode, CodeBlockNode,
  BlockquoteNode, ListNode, ListItemNode, ThematicBreakNode, RawBlockNode,
  // Inline node types
  TextNode, EmphasisNode, StrongNode, CodeSpanNode,
  LinkNode, ImageNode, AutolinkNode, HardBreakNode, SoftBreakNode,
} from "@coding-adventures/document-ast";

// ─── Convenience function ─────────────────────────────────────────────────────

/**
 * Convert an AsciiDoc string directly to an HTML string.
 *
 * This is the most common use case: you have AsciiDoc source and you want
 * HTML output. This function wires together the two constituent packages.
 *
 * @param text   AsciiDoc source string.
 * @returns      HTML string ready for embedding in a web page.
 *
 * @example
 * ```typescript
 * const html = toHtml("= My Doc\n\nHello *world*.\n");
 * // → "<h1>My Doc</h1>\n<p>Hello <strong>world</strong>.</p>\n"
 * ```
 */
export function toHtml(text: string): string {
  const doc = parse(text);
  return renderToHtml(doc);
}

// ─── Version ──────────────────────────────────────────────────────────────────

export const VERSION = "0.1.0";
