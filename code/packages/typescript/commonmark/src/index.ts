/**
 * @coding-adventures/commonmark
 *
 * CommonMark 0.31.2 compliant Markdown parser.
 *
 * Parses Markdown source text into a typed AST (abstract syntax tree) and
 * optionally renders it to HTML. The implementation follows the CommonMark
 * specification exactly, including the two-phase parsing model:
 *
 *   Phase 1 — Block structure: headings, lists, code blocks, blockquotes, etc.
 *   Phase 2 — Inline content:  emphasis, links, images, code spans, etc.
 *
 * === Quick Start ===
 *
 * ```typescript
 * import { parse, toHtml } from "@coding-adventures/commonmark";
 *
 * const ast = parse("# Hello\n\nWorld *with* emphasis.\n");
 * const html = toHtml(ast);
 * // → "<h1>Hello</h1>\n<p>World <em>with</em> emphasis.</p>\n"
 * ```
 *
 * === AST-only usage ===
 *
 * ```typescript
 * import { parse } from "@coding-adventures/commonmark";
 *
 * const doc = parse("## Heading\n\n- item 1\n- item 2\n");
 * for (const block of doc.children) {
 *   console.log(block.type); // "heading", "list"
 * }
 * ```
 *
 * @module index
 */

// ─── Public API ───────────────────────────────────────────────────────────────
//
// NOTE: `toHtml` (html-renderer.ts) is intentionally NOT exported here.
// The `commonmark` package is responsible for parsing only — producing a
// typed AST from Markdown source text. Rendering the AST to HTML, LaTeX,
// PDF, etc. is the responsibility of separate downstream packages:
//
//   @coding-adventures/commonmark-html   — AST → HTML
//   @coding-adventures/commonmark-latex  — AST → LaTeX (future)
//
// The HTML renderer lives in this package ONLY as a test fixture, used
// by the CommonMark spec test suite to verify parsing correctness against
// the 652 spec examples (which compare markdown → HTML).

export type {
  // Node union types
  Node, BlockNode, InlineNode,
  // Block node types
  DocumentNode, HeadingNode, ParagraphNode, CodeBlockNode,
  BlockquoteNode, ListNode, ListItemNode, ThematicBreakNode,
  HtmlBlockNode, LinkDefinitionNode,
  // Inline node types
  TextNode, EmphasisNode, StrongNode, CodeSpanNode,
  LinkNode, ImageNode, AutolinkNode, HtmlInlineNode,
  HardBreakNode, SoftBreakNode,
  // Supporting types
  ParseOptions, LinkReference, LinkRefMap,
} from "./types.js";

// ─── parse() ──────────────────────────────────────────────────────────────────

import { parseBlocks, convertToAst } from "./block-parser.js";
import { resolveInlineContent } from "./inline-parser.js";
import type { DocumentNode, ParseOptions } from "./types.js";

/**
 * Parse a CommonMark Markdown string into a DocumentNode AST.
 *
 * The parse is two-phase:
 *   1. Block parser builds the structural skeleton.
 *   2. Inline parser fills in emphasis, links, and other inline markup.
 *
 * The returned AST is fully resolved — all link references have been
 * expanded, all inline constructs have been parsed. The document is
 * ready for rendering or analysis.
 *
 * @param markdown  The Markdown source string.
 * @param options   Optional parse options (currently only `preset`).
 * @returns         The root DocumentNode.
 *
 * @example
 * ```typescript
 * const doc = parse("# Title\n\nParagraph with **bold** text.\n");
 * doc.type;               // "document"
 * doc.children[0].type;   // "heading"
 * doc.children[1].type;   // "paragraph"
 * ```
 */
export function parse(markdown: string, _options?: ParseOptions): DocumentNode {
  // Phase 1: Block parsing
  const { document: mutableDoc, linkRefs } = parseBlocks(markdown);
  const { document, rawInlineContent } = convertToAst(mutableDoc, linkRefs);

  // Phase 2: Inline parsing
  resolveInlineContent(document, rawInlineContent, linkRefs);

  return document;
}

// ─── Version ──────────────────────────────────────────────────────────────────

export const VERSION = "0.1.0";
