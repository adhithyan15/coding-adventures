/**
 * @coding-adventures/gfm-parser
 *
 * GitHub Flavored Markdown parser.
 *
 * Parses Markdown source text into a Document AST — the format-agnostic IR
 * defined in @coding-adventures/document-ast. The result is a `DocumentNode`
 * ready for any back-end renderer (HTML, PDF, plain text, …).
 *
 * The parse is two-phase:
 *   Phase 1 — Block structure: headings, lists, code blocks, blockquotes, …
 *   Phase 2 — Inline content: emphasis, links, images, code spans, …
 *
 * === Quick Start ===
 *
 * ```typescript
 * import { parse } from "@coding-adventures/gfm-parser";
 *
 * const doc = parse("# Hello\n\nWorld *with* emphasis.\n");
 * doc.type;               // "document"
 * doc.children[0].type;   // "heading"
 * doc.children[1].type;   // "paragraph"
 * ```
 *
 * === With the HTML renderer ===
 *
 * ```typescript
 * import { parse } from "@coding-adventures/gfm-parser";
 * import { toHtml } from "@coding-adventures/document-ast-to-html";
 *
 * const html = toHtml(parse("# Hello\n\nWorld\n"));
 * // → "<h1>Hello</h1>\n<p>World</p>\n"
 * ```
 *
 * @module index
 */

import { parseBlocks, convertToAst, applyGfmBlockExtensions } from "./block-parser.js";
import { resolveInlineContent } from "./inline-parser.js";
import type { DocumentNode } from "@coding-adventures/document-ast";
import type { ParseOptions } from "./types.js";

export type { ParseOptions } from "./types.js";

/**
 * Parse a GitHub Flavored Markdown string into a `DocumentNode` AST.
 *
 * The result conforms to the Document AST spec (TE00) — a format-agnostic IR
 * with all link references resolved and all inline markup parsed.
 *
 * @param markdown  The Markdown source string.
 * @param _options  Optional parse options (reserved; currently unused).
 * @returns         The root `DocumentNode`.
 *
 * @example
 * ```typescript
 * const doc = parse("## Heading\n\n- item 1\n- item 2\n");
 * doc.children[0].type;   // "heading"
 * doc.children[1].type;   // "list"
 * ```
 */
export function parse(markdown: string, _options?: ParseOptions): DocumentNode {
  // Phase 1: Block parsing — builds the structural skeleton
  const { document: mutableDoc, linkRefs } = parseBlocks(markdown);
  const { document, rawInlineContent } = convertToAst(mutableDoc, linkRefs);
  applyGfmBlockExtensions(document, rawInlineContent);

  // Phase 2: Inline parsing — fills in emphasis, links, code spans, etc.
  resolveInlineContent(document, rawInlineContent, linkRefs);

  return document;
}

// ─── Version ──────────────────────────────────────────────────────────────────

export const VERSION = "0.1.0";
