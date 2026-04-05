// ============================================================================
// InlineNode.java — Inline document AST nodes
// ============================================================================

package com.codingadventures.documentast;

import java.util.List;

/**
 * Inline nodes appear inside block nodes that contain prose:
 * headings, paragraphs, list items, and table cells.
 */
public sealed interface InlineNode extends Node
        permits InlineNode.TextNode, InlineNode.EmphasisNode, InlineNode.StrongNode,
                InlineNode.StrikethroughNode, InlineNode.CodeSpanNode, InlineNode.LinkNode,
                InlineNode.ImageNode, InlineNode.AutolinkNode, InlineNode.RawInlineNode,
                InlineNode.HardBreakNode, InlineNode.SoftBreakNode {

    /** Plain text with no markup. Value contains decoded Unicode. */
    record TextNode(String value) implements InlineNode {
        @Override public String nodeType() { return "text"; }
    }

    /** Stressed emphasis — renders as <em> / italic. */
    record EmphasisNode(List<InlineNode> children) implements InlineNode {
        @Override public String nodeType() { return "emphasis"; }
    }

    /** Strong importance — renders as <strong> / bold. */
    record StrongNode(List<InlineNode> children) implements InlineNode {
        @Override public String nodeType() { return "strong"; }
    }

    /** Struck-through text. */
    record StrikethroughNode(List<InlineNode> children) implements InlineNode {
        @Override public String nodeType() { return "strikethrough"; }
    }

    /** Inline code span. Value is raw, not decoded for HTML entities. */
    record CodeSpanNode(String value) implements InlineNode {
        @Override public String nodeType() { return "code_span"; }
    }

    /**
     * A hyperlink with resolved destination.
     *
     * @param destination fully resolved URL
     * @param title       optional tooltip text (null if absent)
     */
    record LinkNode(String destination, String title,
                    List<InlineNode> children) implements InlineNode {
        @Override public String nodeType() { return "link"; }
    }

    /**
     * An embedded image.
     *
     * @param destination fully resolved URL
     * @param title       optional tooltip text (null if absent)
     * @param alt          plain-text alt description (markup stripped)
     */
    record ImageNode(String destination, String title, String alt) implements InlineNode {
        @Override public String nodeType() { return "image"; }
    }

    /**
     * A URL or email address presented as a direct link.
     *
     * @param destination the URL or email address
     * @param isEmail     true for email autolinks
     */
    record AutolinkNode(String destination, boolean isEmail) implements InlineNode {
        @Override public String nodeType() { return "autolink"; }
    }

    /** Raw inline content for a specific back-end format. */
    record RawInlineNode(String format, String value) implements InlineNode {
        @Override public String nodeType() { return "raw_inline"; }
    }

    /** Forced line break within a paragraph. */
    record HardBreakNode() implements InlineNode {
        @Override public String nodeType() { return "hard_break"; }
    }

    /** Soft line break — a newline that is not a hard break. */
    record SoftBreakNode() implements InlineNode {
        @Override public String nodeType() { return "soft_break"; }
    }
}
