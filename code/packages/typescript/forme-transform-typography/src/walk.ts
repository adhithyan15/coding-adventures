/**
 * walk.ts — AST walker producing a typography-transformed copy
 * of a `DocumentNode`.
 *
 * The `document-ast` IR is immutable by contract (every field
 * `readonly`), so applying any transformation requires
 * producing a fresh tree.  This walker:
 *
 *   1. Descends every block / inline container.
 *   2. For each `TextNode`, returns a NEW `TextNode` whose
 *      `value` has been run through `typeset`.
 *   3. For all other nodes, returns a new node with the same
 *      fields but possibly-transformed children (deep copy of
 *      shape; primitive fields are shared since they're
 *      immutable).
 *
 * Why not just walk and mutate?  Two reasons:
 *
 *   - **Contract.**  The IR is `readonly`; mutation would
 *     silently violate the type contract and break any
 *     downstream consumer that holds a reference to the original
 *     `doc`.
 *   - **Reproducibility.**  Producing a fresh tree per call
 *     means the function is genuinely pure — same input →
 *     byte-identical (and identity-fresh) output.  Safe to use
 *     as a cache key derivation step.
 *
 * Nodes whose payload is NOT prose text (code blocks, code spans,
 * raw HTML, image alt text, link URLs) are passed through
 * unchanged.  Smart-quoting code samples would break syntax.
 *
 * @module walk
 */

import type {
  BlockNode,
  CodeBlockNode,
  CodeSpanNode,
  DocumentNode,
  ImageNode,
  InlineNode,
  LinkNode,
  ListChildNode,
  RawBlockNode,
  RawInlineNode,
  TextNode,
  ThematicBreakNode,
  TableNode,
  TableRowNode,
  TableCellNode,
} from "@coding-adventures/document-ast";
import { typeset } from "./typeset.js";
import type { TypographyOptions } from "./types.js";

/**
 * Produce a typography-transformed copy of `doc`.  Input is
 * never mutated.
 *
 * Default options: `smartQuotes`, `dashes`, `ellipsis` enabled;
 * `ligatures` disabled.
 *
 * Pass-through nodes (NOT typeset):
 *   - `CodeBlockNode.value` — would break source code syntax.
 *   - `CodeSpanNode.value` — same reason for inline code.
 *   - `RawBlockNode.value` / `RawInlineNode.value` — by
 *     definition the renderer wants verbatim output.
 *   - `LinkNode.destination` / `ImageNode.destination` — URLs
 *     mustn't get smart-quoted.
 *   - `ImageNode.alt` — passthrough by default (might be debatable;
 *     v0 chooses safety).
 */
export function typography(
  doc: DocumentNode,
  options: TypographyOptions = {},
): DocumentNode {
  return {
    type: "document",
    children: doc.children.map((b) => transformBlock(b, options)),
  };
}

function transformBlock(b: BlockNode, options: TypographyOptions): BlockNode {
  switch (b.type) {
    case "document":
      return {
        type: "document",
        children: b.children.map((c) => transformBlock(c, options)),
      };
    case "heading":
      return {
        type: "heading",
        level: b.level,
        children: b.children.map((c) => transformInline(c, options)),
      };
    case "paragraph":
      return {
        type: "paragraph",
        children: b.children.map((c) => transformInline(c, options)),
      };
    case "blockquote":
      return {
        type: "blockquote",
        children: b.children.map((c) => transformBlock(c, options)),
      };
    case "list":
      return {
        type: "list",
        ordered: b.ordered,
        start: b.start,
        tight: b.tight,
        children: b.children.map((c) => transformListChild(c, options)),
      };
    case "list_item":
      return {
        type: "list_item",
        children: b.children.map((c) => transformBlock(c, options)),
      };
    case "task_item":
      return {
        type: "task_item",
        checked: b.checked,
        children: b.children.map((c) => transformBlock(c, options)),
      };
    case "table":
      return transformTable(b, options);
    case "table_row":
      return transformTableRow(b, options);
    case "table_cell":
      return transformTableCell(b, options);
    // Pass-through (no prose text to typeset).
    case "code_block":
      return passthroughCodeBlock(b);
    case "thematic_break":
      return passthroughThematicBreak(b);
    case "raw_block":
      return passthroughRawBlock(b);
    default: {
      const _exhaustive: never = b;
      void _exhaustive;
      return b;
    }
  }
}

function transformInline(n: InlineNode, options: TypographyOptions): InlineNode {
  switch (n.type) {
    case "text":
      return transformText(n, options);
    case "emphasis":
      return {
        type: "emphasis",
        children: n.children.map((c) => transformInline(c, options)),
      };
    case "strong":
      return {
        type: "strong",
        children: n.children.map((c) => transformInline(c, options)),
      };
    case "strikethrough":
      return {
        type: "strikethrough",
        children: n.children.map((c) => transformInline(c, options)),
      };
    case "link":
      return passthroughLink(n, options);
    case "image":
      return passthroughImage(n);
    case "code_span":
      return passthroughCodeSpan(n);
    case "autolink":
      return { type: "autolink", destination: n.destination, isEmail: n.isEmail };
    case "raw_inline":
      return passthroughRawInline(n);
    case "hard_break":
      return { type: "hard_break" };
    case "soft_break":
      return { type: "soft_break" };
    default: {
      const _exhaustive: never = n;
      void _exhaustive;
      return n;
    }
  }
}

function transformListChild(c: ListChildNode, options: TypographyOptions): ListChildNode {
  if (c.type === "task_item") {
    return {
      type: "task_item",
      checked: c.checked,
      children: c.children.map((b) => transformBlock(b, options)),
    };
  }
  return {
    type: "list_item",
    children: c.children.map((b) => transformBlock(b, options)),
  };
}

function transformTable(t: TableNode, options: TypographyOptions): TableNode {
  return {
    type: "table",
    align: t.align,
    children: t.children.map((r) => transformTableRow(r, options)),
  };
}

function transformTableRow(r: TableRowNode, options: TypographyOptions): TableRowNode {
  return {
    type: "table_row",
    isHeader: r.isHeader,
    children: r.children.map((c) => transformTableCell(c, options)),
  };
}

function transformTableCell(c: TableCellNode, options: TypographyOptions): TableCellNode {
  return {
    type: "table_cell",
    children: c.children.map((inline) => transformInline(inline, options)),
  };
}

function transformText(t: TextNode, options: TypographyOptions): TextNode {
  return { type: "text", value: typeset(t.value, options) };
}

// ─── Pass-through node copies ──────────────────────────────────────
//
// Each one returns a fresh object with the same primitive field
// values.  This way the public API guarantee "fresh tree every
// call" holds even for sub-trees the typography pass doesn't
// touch.

function passthroughCodeBlock(b: CodeBlockNode): CodeBlockNode {
  return { type: "code_block", language: b.language, value: b.value };
}

function passthroughThematicBreak(b: ThematicBreakNode): ThematicBreakNode {
  return { type: "thematic_break" };
}

function passthroughRawBlock(b: RawBlockNode): RawBlockNode {
  return { type: "raw_block", format: b.format, value: b.value };
}

function passthroughLink(n: LinkNode, options: TypographyOptions): LinkNode {
  // URL passes through unchanged; child label is typeset.
  return {
    type: "link",
    destination: n.destination,
    title: n.title,
    children: n.children.map((c) => transformInline(c, options)),
  };
}

function passthroughImage(n: ImageNode): ImageNode {
  return { type: "image", destination: n.destination, title: n.title, alt: n.alt };
}

function passthroughCodeSpan(n: CodeSpanNode): CodeSpanNode {
  return { type: "code_span", value: n.value };
}

function passthroughRawInline(n: RawInlineNode): RawInlineNode {
  return { type: "raw_inline", format: n.format, value: n.value };
}
