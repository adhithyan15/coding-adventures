// ============================================================================
// DocumentAst.kt — Universal document IR types
// ============================================================================
//
// The Document AST is a format-agnostic intermediate representation for
// structured documents. It sits between source formats (Markdown, RST, HTML)
// and output renderers (HTML, PDF, plain text).
//
// Kotlin's sealed interfaces provide exhaustive when-expressions, ensuring
// every node type is handled at compile time.
//
// Design principles:
//   1. Semantic, not notational — nodes carry meaning, not syntax
//   2. Resolved, not deferred — all references resolved before IR
//   3. Format-agnostic — RawBlock/RawInline carry format tags
//   4. Minimal and stable — only universal document concepts
//
// Layer: TE00 (text/language layer — document IR)
// ============================================================================

package com.codingadventures.documentast

// ---------------------------------------------------------------------------
// Root interface
// ---------------------------------------------------------------------------

/** Root interface for all Document AST nodes. */
sealed interface Node {
    val nodeType: String
}

// ---------------------------------------------------------------------------
// Block nodes
// ---------------------------------------------------------------------------

/** Block-level nodes form the structural skeleton of a document. */
sealed interface BlockNode : Node

/** Root of every document. */
data class DocumentNode(val children: List<BlockNode>) : BlockNode {
    override val nodeType get() = "document"
}

/** Section heading with depth 1-6. */
data class HeadingNode(val level: Int, val children: List<InlineNode>) : BlockNode {
    override val nodeType get() = "heading"
}

/** A block of prose containing inline nodes. */
data class ParagraphNode(val children: List<InlineNode>) : BlockNode {
    override val nodeType get() = "paragraph"
}

/** A block of literal code. */
data class CodeBlockNode(val language: String, val value: String) : BlockNode {
    override val nodeType get() = "code_block"
}

/** Quotation or aside. Can contain nested blocks. */
data class BlockquoteNode(val children: List<BlockNode>) : BlockNode {
    override val nodeType get() = "blockquote"
}

/** Ordered or unordered list. */
data class ListNode(
    val ordered: Boolean,
    val start: Int,
    val tight: Boolean,
    val children: List<BlockNode>
) : BlockNode {
    override val nodeType get() = "list"
}

/** One item in a list. */
data class ListItemNode(val children: List<BlockNode>) : BlockNode {
    override val nodeType get() = "list_item"
}

/** Task-list item with checkbox. */
data class TaskItemNode(val checked: Boolean, val children: List<BlockNode>) : BlockNode {
    override val nodeType get() = "task_item"
}

/** Horizontal rule. Leaf node. */
data object ThematicBreakNode : BlockNode {
    override val nodeType get() = "thematic_break"
}

/** Raw content for a specific back-end format. */
data class RawBlockNode(val format: String, val value: String) : BlockNode {
    override val nodeType get() = "raw_block"
}

/** Table with column alignments and rows. */
data class TableNode(val align: List<TableAlignment>, val rows: List<TableRowNode>) : BlockNode {
    override val nodeType get() = "table"
}

/** One row in a table. */
data class TableRowNode(val isHeader: Boolean, val cells: List<TableCellNode>) : BlockNode {
    override val nodeType get() = "table_row"
}

/** Single table cell. */
data class TableCellNode(val children: List<InlineNode>) : BlockNode {
    override val nodeType get() = "table_cell"
}

/** Column alignment for tables. */
enum class TableAlignment { NONE, LEFT, RIGHT, CENTER }

// ---------------------------------------------------------------------------
// Inline nodes
// ---------------------------------------------------------------------------

/** Inline nodes appear inside block nodes that contain prose. */
sealed interface InlineNode : Node

/** Plain text with no markup. */
data class TextNode(val value: String) : InlineNode {
    override val nodeType get() = "text"
}

/** Stressed emphasis (italic). */
data class EmphasisNode(val children: List<InlineNode>) : InlineNode {
    override val nodeType get() = "emphasis"
}

/** Strong importance (bold). */
data class StrongNode(val children: List<InlineNode>) : InlineNode {
    override val nodeType get() = "strong"
}

/** Struck-through text. */
data class StrikethroughNode(val children: List<InlineNode>) : InlineNode {
    override val nodeType get() = "strikethrough"
}

/** Inline code span. */
data class CodeSpanNode(val value: String) : InlineNode {
    override val nodeType get() = "code_span"
}

/** Hyperlink with resolved destination. */
data class LinkNode(
    val destination: String,
    val title: String?,
    val children: List<InlineNode>
) : InlineNode {
    override val nodeType get() = "link"
}

/** Embedded image. */
data class ImageNode(
    val destination: String,
    val title: String?,
    val alt: String
) : InlineNode {
    override val nodeType get() = "image"
}

/** URL or email autolink. */
data class AutolinkNode(val destination: String, val isEmail: Boolean) : InlineNode {
    override val nodeType get() = "autolink"
}

/** Raw inline content for a specific back-end. */
data class RawInlineNode(val format: String, val value: String) : InlineNode {
    override val nodeType get() = "raw_inline"
}

/** Forced line break. */
data object HardBreakNode : InlineNode {
    override val nodeType get() = "hard_break"
}

/** Soft line break. */
data object SoftBreakNode : InlineNode {
    override val nodeType get() = "soft_break"
}
