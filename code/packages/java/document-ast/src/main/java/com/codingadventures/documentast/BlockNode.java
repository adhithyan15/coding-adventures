// ============================================================================
// BlockNode.java — Block-level document AST nodes
// ============================================================================

package com.codingadventures.documentast;

import java.util.List;

/**
 * Block-level nodes form the structural skeleton of a document.
 * They live inside DocumentNode, BlockquoteNode, and ListItemNode.
 */
public sealed interface BlockNode extends Node
        permits BlockNode.DocumentNode, BlockNode.HeadingNode, BlockNode.ParagraphNode,
                BlockNode.CodeBlockNode, BlockNode.BlockquoteNode, BlockNode.ListNode,
                BlockNode.ListItemNode, BlockNode.TaskItemNode, BlockNode.ThematicBreakNode,
                BlockNode.RawBlockNode, BlockNode.TableNode, BlockNode.TableRowNode,
                BlockNode.TableCellNode {

    /** Root of every document. An empty document has an empty children list. */
    record DocumentNode(List<BlockNode> children) implements BlockNode {
        @Override public String nodeType() { return "document"; }
    }

    /** Section heading with nesting depth 1-6. */
    record HeadingNode(int level, List<InlineNode> children) implements BlockNode {
        @Override public String nodeType() { return "heading"; }
    }

    /** A block of prose containing inline nodes. */
    record ParagraphNode(List<InlineNode> children) implements BlockNode {
        @Override public String nodeType() { return "paragraph"; }
    }

    /** A block of literal code or pre-formatted text. */
    record CodeBlockNode(String language, String value) implements BlockNode {
        @Override public String nodeType() { return "code_block"; }
    }

    /** A block of content set apart as a quotation. Can contain nested blocks. */
    record BlockquoteNode(List<BlockNode> children) implements BlockNode {
        @Override public String nodeType() { return "blockquote"; }
    }

    /**
     * An ordered or unordered list.
     *
     * @param ordered true for numbered lists
     * @param start   opening number for ordered lists (default 1; 0 for unordered)
     * @param tight   true if no blank lines between items (rendering hint)
     */
    record ListNode(boolean ordered, int start, boolean tight,
                    List<BlockNode> children) implements BlockNode {
        @Override public String nodeType() { return "list"; }
    }

    /** One item in a ListNode. Contains block-level content. */
    record ListItemNode(List<BlockNode> children) implements BlockNode {
        @Override public String nodeType() { return "list_item"; }
    }

    /** A task-list item with a checkbox state. */
    record TaskItemNode(boolean checked, List<BlockNode> children) implements BlockNode {
        @Override public String nodeType() { return "task_item"; }
    }

    /** A horizontal rule / thematic break. Leaf node, no children. */
    record ThematicBreakNode() implements BlockNode {
        @Override public String nodeType() { return "thematic_break"; }
    }

    /**
     * Raw content for a specific back-end (e.g. "html", "latex").
     * Back-ends that don't recognise the format skip this node silently.
     */
    record RawBlockNode(String format, String value) implements BlockNode {
        @Override public String nodeType() { return "raw_block"; }
    }

    /** A table with column alignments and rows. */
    record TableNode(List<TableAlignment> align,
                     List<TableRowNode> rows) implements BlockNode {
        @Override public String nodeType() { return "table"; }
    }

    /** One row in a table. */
    record TableRowNode(boolean isHeader,
                        List<TableCellNode> cells) implements BlockNode {
        @Override public String nodeType() { return "table_row"; }
    }

    /** A single table cell containing inline content. */
    record TableCellNode(List<InlineNode> children) implements BlockNode {
        @Override public String nodeType() { return "table_cell"; }
    }
}
