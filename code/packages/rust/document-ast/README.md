# document-ast

Format-agnostic Intermediate Representation (IR) for structured documents — the "LLVM IR of documents".

## What is it?

`document-ast` defines the Document AST: a tree of typed nodes that represents the semantic structure of a document, independent of any source format (Markdown, HTML, reStructuredText, …) or output format (HTML, PDF, plain text, …).

```text
                    ┌─────────────────────────┐
  Markdown source   │   commonmark-parser      │   parse(markdown)
  ─────────────────▶│  (or any other parser)   │──────────────────▶  DocumentNode
                    └─────────────────────────┘
                                                          │
                                                          ▼
                    ┌─────────────────────────┐   DocumentNode
                    │  document-ast-to-html    │◀──────────────────
  HTML output       │  (or PDF, plain text…)  │   to_html(&doc)
  ◀─────────────────│                         │
                    └─────────────────────────┘
```

## Node types

### Block nodes

| Type | Description |
|------|-------------|
| `DocumentNode` | Root of the tree — holds a list of block nodes |
| `HeadingNode` | ATX (`# …`) or setext heading, level 1–6 |
| `ParagraphNode` | Paragraph of inline content |
| `CodeBlockNode` | Fenced or indented code block, optional language |
| `BlockquoteNode` | `> …` blockquote, holds nested blocks |
| `ListNode` | Ordered (`<ol>`) or unordered (`<ul>`) list |
| `ListItemNode` | Single item in a list |
| `ThematicBreakNode` | Horizontal rule `---` / `***` / `___` |
| `RawBlockNode` | Verbatim block (e.g. raw HTML), format-tagged |

### Inline nodes

| Type | Description |
|------|-------------|
| `TextNode` | Plain text |
| `EmphasisNode` | `*em*` / `_em_` |
| `StrongNode` | `**strong**` / `__strong__` |
| `CodeSpanNode` | `` `code` `` |
| `LinkNode` | `[text](url "title")` |
| `ImageNode` | `![alt](url "title")` |
| `AutolinkNode` | `<url>` / `<email>` |
| `RawInlineNode` | Verbatim inline (e.g. raw HTML), format-tagged |
| `HardBreakNode` | Hard line break `  \n` |
| `SoftBreakNode` | Soft line break (newline in paragraph) |

## Usage

```rust
use document_ast::{DocumentNode, BlockNode, ParagraphNode, InlineNode, TextNode};

let doc = DocumentNode {
    children: vec![
        BlockNode::Paragraph(ParagraphNode {
            children: vec![
                InlineNode::Text(TextNode { value: "Hello world".into() }),
            ],
        }),
    ],
};
println!("{} block(s)", doc.children.len()); // "1 block(s)"
```

## How it fits in the stack

```text
document-ast           ← you are here — shared types
      ↓ types                 ↓ types
commonmark-parser      document-ast-to-html
parse(markdown)        to_html(&doc)
      ↓ depends on both
commonmark             ← convenience crate
markdown_to_html(md)
```

This crate has no dependencies — it is purely type definitions. Every parser and renderer depends on it, but it depends on nothing.

## Version

0.1.0
