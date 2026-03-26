/**
 * @coding-adventures/gfm
 *
 * GitHub Flavored Markdown pipeline convenience package.
 *
 * This is the public-facing convenience package that combines two constituent
 * packages into a simple two-function API:
 *
 *   - `parse(markdown)` — from @coding-adventures/gfm-parser
 *   - `toHtml(doc)`    — from @coding-adventures/document-ast-to-html
 *
 * ```
 *   @coding-adventures/document-ast        ← format-agnostic types
 *          ↓ types                                ↓ types
 *   @coding-adventures/gfm-parser          @coding-adventures/document-ast-to-html
 *     parse(markdown) → DocumentNode         toHtml(doc) → string
 *          ↓ depends on both
 *   @coding-adventures/gfm                  ← you are here
 *     const html = toHtml(parse(markdown))
 * ```
 *
 * Users who just want to convert Markdown to HTML:
 *
 * ```typescript
 * import { parse, toHtml } from "@coding-adventures/gfm";
 *
 * const html = toHtml(parse("# Hello\n\nWorld *with* emphasis.\n"));
 * // → "<h1>Hello</h1>\n<p>World <em>with</em> emphasis.</p>\n"
 * ```
 *
 * Users who want to work with the AST directly, plug in a different renderer,
 * or build a Markdown → PDF pipeline should import the constituent packages:
 *
 * ```typescript
 * import { parse }  from "@coding-adventures/gfm-parser";
 * import { toHtml } from "@coding-adventures/document-ast-to-html";
 * import type { DocumentNode } from "@coding-adventures/document-ast";
 *
 * const doc: DocumentNode = parse(markdownString);
 * const html = toHtml(doc);
 * ```
 *
 * @module index
 */

// ─── Re-exports from constituent packages ────────────────────────────────────

export { parse } from "@coding-adventures/gfm-parser";
export type { ParseOptions } from "@coding-adventures/gfm-parser";

export { toHtml } from "@coding-adventures/document-ast-to-html";

export type {
  // Node union types
  Node, BlockNode, InlineNode,
  // Block node types
  DocumentNode, HeadingNode, ParagraphNode, CodeBlockNode,
  BlockquoteNode, ListNode, ListItemNode, TaskItemNode, ThematicBreakNode,
  RawBlockNode, TableNode, TableRowNode, TableCellNode, TableAlignment,
  // Inline node types
  TextNode, EmphasisNode, StrongNode, StrikethroughNode, CodeSpanNode,
  LinkNode, ImageNode, AutolinkNode, RawInlineNode,
  HardBreakNode, SoftBreakNode,
} from "@coding-adventures/document-ast";

// ─── Version ──────────────────────────────────────────────────────────────────

export const VERSION = "0.1.0";
