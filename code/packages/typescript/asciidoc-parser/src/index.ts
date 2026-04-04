/**
 * @coding-adventures/asciidoc-parser
 *
 * AsciiDoc parser — produces a Document AST from AsciiDoc source text.
 *
 * The parse is two-phase:
 *   Phase 1 — Block structure: headings, lists, code blocks, blockquotes, …
 *   Phase 2 — Inline content: emphasis (underscore), strong (asterisk),
 *             code spans, links, images, line breaks, …
 *
 * === Key AsciiDoc vs CommonMark differences ===
 *
 * | Syntax   | CommonMark        | AsciiDoc          |
 * |----------|-------------------|-------------------|
 * | `*text*` | EmphasisNode      | StrongNode (bold) |
 * | `_text_` | EmphasisNode      | EmphasisNode      |
 * | `**t**`  | StrongNode        | StrongNode (uncon)|
 * | `__t__`  | Not standard      | EmphasisNode (unc)|
 * | headings | `# H1`, `## H2`   | `= H1`, `== H2`   |
 * | code     | ` ``` ` fences    | `----` delimiters |
 * | quote    | `> ` prefix       | `____` delimiters |
 * | break    | `'...'` 3 apostrophes | `'''` thematic break |
 *
 * The result is a standard DocumentNode that any back-end renderer
 * (HTML, PDF, plain text) can consume.
 *
 * === Quick Start ===
 *
 * ```typescript
 * import { parse } from "@coding-adventures/asciidoc-parser";
 *
 * const doc = parse("= Hello\n\nWorld *bold*.\n");
 * doc.type;              // "document"
 * doc.children[0].type; // "heading"
 * doc.children[1].type; // "paragraph"
 * ```
 *
 * @module index
 */

export { parse } from "./block-parser.js";
export { parseInline } from "./inline-parser.js";

export type {
  DocumentNode,
  BlockNode,
  InlineNode,
  HeadingNode,
  ParagraphNode,
  CodeBlockNode,
  BlockquoteNode,
  ListNode,
  ListItemNode,
  ThematicBreakNode,
  RawBlockNode,
  TextNode,
  EmphasisNode,
  StrongNode,
  CodeSpanNode,
  LinkNode,
  ImageNode,
  AutolinkNode,
  HardBreakNode,
  SoftBreakNode,
} from "@coding-adventures/document-ast";

export const VERSION = "0.1.0";
