"""Document AST — Format-Agnostic Intermediate Representation for structured documents.

The Document AST is the "LLVM IR of documents" — a stable, typed, immutable tree
that every front-end parser produces and every back-end renderer consumes.

    Markdown ──────────────────────────────► HTML
    reStructuredText ───► Document AST ───► PDF
    HTML ──────────────────────────────────► Plain text
    DOCX ──────────────────────────────────► DOCX

N front-ends × M back-ends requires only N + M implementations instead of N × M.

This is a **types-only** package — there is no runtime code beyond the type
definitions. Import it to annotate AST values produced by a front-end parser
or consumed by a back-end renderer.

=== Key design decisions ===

**No LinkDefinitionNode** — links in the IR are always fully resolved.
Markdown's [text][label] reference syntax is resolved by the front-end;
the IR only ever contains LinkNode { destination: "..." }.

**RawBlockNode / RawInlineNode** instead of HtmlBlockNode / HtmlInlineNode.
A `format` field ("html", "latex", ...) identifies the target back-end.
Renderers skip nodes with an unknown `format`.

Spec: TE00 — Document AST
"""

from coding_adventures_document_ast.types import (
    AutolinkNode,
    BlockNode,
    BlockquoteNode,
    CodeBlockNode,
    CodeSpanNode,
    # Block node types
    DocumentNode,
    EmphasisNode,
    HardBreakNode,
    HeadingNode,
    ImageNode,
    InlineNode,
    LinkNode,
    ListItemNode,
    ListNode,
    # Node union types
    Node,
    ParagraphNode,
    RawBlockNode,
    RawInlineNode,
    SoftBreakNode,
    StrongNode,
    # Inline node types
    TextNode,
    ThematicBreakNode,
)

__all__ = [
    "Node",
    "BlockNode",
    "InlineNode",
    "DocumentNode",
    "HeadingNode",
    "ParagraphNode",
    "CodeBlockNode",
    "BlockquoteNode",
    "ListNode",
    "ListItemNode",
    "ThematicBreakNode",
    "RawBlockNode",
    "TextNode",
    "EmphasisNode",
    "StrongNode",
    "CodeSpanNode",
    "LinkNode",
    "ImageNode",
    "AutolinkNode",
    "RawInlineNode",
    "HardBreakNode",
    "SoftBreakNode",
]
