// ============================================================================
// Node.java — Document AST node type hierarchy
// ============================================================================
//
// The Document AST is the "LLVM IR of documents" — a format-agnostic
// intermediate representation for structured documents. Just as LLVM IR sits
// between many source languages and many targets, the Document AST sits
// between document source formats (Markdown, RST, HTML) and output renderers
// (HTML, PDF, plain text):
//
//   Markdown ──────────────────────────────► HTML
//   reStructuredText ──► Document AST ──────► PDF
//   HTML ───────────────────────────────────► Plain text
//
// With this shared IR, N front-ends x M back-ends requires only N+M
// implementations instead of N*M.
//
// Design principles:
//
//   1. Semantic, not notational — nodes carry meaning (heading, emphasis),
//      not syntax (### or ***).
//
//   2. Resolved, not deferred — all link references are resolved before
//      the IR is produced.
//
//   3. Format-agnostic — RawBlock and RawInline carry a format tag so
//      back-ends can selectively pass through or ignore content.
//
//   4. Minimal and stable — only universal document concepts appear here.
//
// The type hierarchy uses sealed interfaces for exhaustive pattern matching
// in Java 21+:
//
//   Node (sealed)
//     BlockNode (sealed) — structural nodes
//       DocumentNode, HeadingNode, ParagraphNode, CodeBlockNode,
//       BlockquoteNode, ListNode, ListItemNode, TaskItemNode,
//       ThematicBreakNode, RawBlockNode, TableNode, TableRowNode, TableCellNode
//     InlineNode (sealed) — inline content nodes
//       TextNode, EmphasisNode, StrongNode, StrikethroughNode,
//       CodeSpanNode, LinkNode, ImageNode, AutolinkNode,
//       RawInlineNode, HardBreakNode, SoftBreakNode
//
// Layer: TE00 (text/language layer — document IR)
// ============================================================================

package com.codingadventures.documentast;

/**
 * Root interface for all Document AST nodes.
 *
 * <p>Use pattern matching (instanceof or switch) to access concrete types.
 */
public sealed interface Node
        permits BlockNode, InlineNode {

    /** Returns a string tag for the node type (e.g. "heading", "paragraph"). */
    String nodeType();
}
