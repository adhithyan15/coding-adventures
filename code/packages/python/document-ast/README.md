# coding-adventures-document-ast

Format-agnostic Intermediate Representation (IR) for structured documents.

## What is this?

The Document AST is the "LLVM IR of documents" — a stable, typed, immutable tree
that every front-end parser produces and every back-end renderer consumes. With a
shared IR, N front-ends × M back-ends requires only N + M implementations instead
of N × M.

```
Markdown ──────────────────────────────► HTML
reStructuredText ───► Document AST ───► PDF
HTML ──────────────────────────────────► Plain text
DOCX ──────────────────────────────────► DOCX
```

## Quick Start

```python
from coding_adventures_document_ast import DocumentNode, HeadingNode, TextNode

# Build a document programmatically
doc: DocumentNode = {
    "type": "document",
    "children": [
        {
            "type": "heading",
            "level": 1,
            "children": [{"type": "text", "value": "Hello"}],
        }
    ],
}
```

## Node Types

### Block Nodes

| Type | Description |
|------|-------------|
| `DocumentNode` | Root of every document |
| `HeadingNode` | Section heading (level 1–6) |
| `ParagraphNode` | Block of prose |
| `CodeBlockNode` | Fenced or indented code block |
| `BlockquoteNode` | Quoted content |
| `ListNode` | Ordered or unordered list |
| `ListItemNode` | Single list item |
| `ThematicBreakNode` | Horizontal rule / `<hr />` |
| `RawBlockNode` | Raw pass-through content (e.g. HTML blocks) |

### Inline Nodes

| Type | Description |
|------|-------------|
| `TextNode` | Plain text (entities already decoded) |
| `EmphasisNode` | `<em>` emphasis |
| `StrongNode` | `<strong>` strong importance |
| `CodeSpanNode` | Inline code |
| `LinkNode` | Hyperlink (destination always resolved) |
| `ImageNode` | Embedded image |
| `AutolinkNode` | URL or email autolink |
| `RawInlineNode` | Raw inline pass-through (e.g. inline HTML) |
| `HardBreakNode` | `<br />` hard line break |
| `SoftBreakNode` | Soft line break (newline within paragraph) |

## Key Design Decisions

**No `LinkDefinitionNode`** — links in the IR are always fully resolved.
Markdown's `[text][label]` reference syntax is resolved by the front-end;
the IR only ever contains `LinkNode { destination: "..." }`.

**`RawBlockNode` / `RawInlineNode`** instead of `HtmlBlockNode` / `HtmlInlineNode`.
A `format` field identifies the target back-end. Renderers skip nodes with an
unknown `format`.

## Spec

This package implements TE00 — Document AST.
