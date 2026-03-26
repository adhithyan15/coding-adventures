/**
 * @coding-adventures/document-ast
 *
 * Format-agnostic Intermediate Representation (IR) for structured documents.
 *
 * The Document AST is the "LLVM IR of documents" — a stable, typed,
 * immutable tree that every front-end parser produces and every back-end
 * renderer consumes. With a shared IR, N front-ends × M back-ends requires
 * only N + M implementations instead of N × M.
 *
 * ```
 *   Markdown ────────────────────────────────► HTML
 *   reStructuredText ────► Document AST ────► PDF
 *   HTML ────────────────────────────────────► Plain text
 *   DOCX ────────────────────────────────────► DOCX
 * ```
 *
 * This is a **types-only** package — there is no runtime code and no
 * dependencies. Import it to annotate the AST values produced by a front-end
 * or consumed by a back-end.
 *
 * === Quick Start ===
 *
 * ```typescript
 * import type { DocumentNode, BlockNode, InlineNode } from "@coding-adventures/document-ast";
 *
 * function countHeadings(doc: DocumentNode): number {
 *   return doc.children.filter(n => n.type === "heading").length;
 * }
 * ```
 *
 * === Key design decisions ===
 *
 * **No `LinkDefinitionNode`** — links in the IR are always fully resolved.
 * Markdown's `[text][label]` reference syntax is resolved by the front-end;
 * the IR only ever contains `LinkNode { destination: "…" }`.
 *
 * **`RawBlockNode` / `RawInlineNode`** instead of `HtmlBlockNode` /
 * `HtmlInlineNode`. A `format` field (`"html"`, `"latex"`, …) identifies the
 * target back-end. Renderers skip nodes with an unknown `format`.
 *
 * Spec: TE00 — Document AST
 *
 * @module index
 */

export type {
  // Node union types
  Node,
  BlockNode,
  InlineNode,
  // Block node types
  DocumentNode,
  HeadingNode,
  ParagraphNode,
  CodeBlockNode,
  BlockquoteNode,
  ListNode,
  ListItemNode,
  ListChildNode,
  TaskItemNode,
  ThematicBreakNode,
  RawBlockNode,
  TableNode,
  TableRowNode,
  TableCellNode,
  TableAlignment,
  // Inline node types
  TextNode,
  EmphasisNode,
  StrongNode,
  StrikethroughNode,
  CodeSpanNode,
  LinkNode,
  ImageNode,
  AutolinkNode,
  RawInlineNode,
  HardBreakNode,
  SoftBreakNode,
} from "./types.js";
