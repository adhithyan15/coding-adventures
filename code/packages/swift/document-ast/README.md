# document-ast (Swift)

Format-agnostic Intermediate Representation (IR) for structured documents.

## Overview

`document-ast` defines the **Document AST** — the shared IR between all document parsers and renderers in this project. It is the "LLVM IR of documents": every front-end parser (Markdown, RST, HTML) produces this IR, and every back-end renderer (HTML, PDF, plain text) consumes it.

With a shared IR, **N parsers × M renderers** requires only **N + M** implementations instead of **N × M**:

```
Markdown ─────────────────────────────────► HTML
reStructuredText ──► Document AST (IR) ──► PDF
HTML input ───────────────────────────────► Plain text
DOCX ─────────────────────────────────────► LaTeX
```

## Design Principles

1. **Semantic, not notational** — nodes carry meaning, not syntax. A `HeadingNode` represents a section heading regardless of whether the source was ATX (`# Foo`) or setext (`Foo\n===`).
2. **Resolved, not deferred** — all link references are resolved by the front-end parser before the IR is emitted. The IR never contains unresolved `[label]` references.
3. **Format-agnostic** — instead of HTML-specific `html_block` nodes, the IR uses `RawBlockNode` with a `format` tag. Back-ends skip nodes with an unknown format.
4. **Minimal and stable** — only concepts that appear in essentially all document formats.

## Node Types

### Block Nodes (structural)

| Case | Description |
|------|-------------|
| `.document(DocumentNode)` | Root of the document |
| `.heading(HeadingNode)` | Section heading (h1–h6) |
| `.paragraph(ParagraphNode)` | Paragraph of inline content |
| `.codeBlock(CodeBlockNode)` | Fenced or indented code block |
| `.blockquote(BlockquoteNode)` | Block quotation |
| `.list(ListNode)` | Ordered or unordered list |
| `.listItem(ListItemNode)` | One list item |
| `.taskItem(TaskItemNode)` | GFM checkbox list item |
| `.thematicBreak` | Horizontal rule |
| `.rawBlock(RawBlockNode)` | Format-specific passthrough |
| `.table(TableNode)` | Tabular data |
| `.tableRow(TableRowNode)` | One table row |
| `.tableCell(TableCellNode)` | One table cell |

### Inline Nodes (character-level)

| Case | Description |
|------|-------------|
| `.text(TextNode)` | Plain text |
| `.emphasis(EmphasisNode)` | Italic (`<em>`) |
| `.strong(StrongNode)` | Bold (`<strong>`) |
| `.codeSpan(CodeSpanNode)` | Inline code (`<code>`) |
| `.link(LinkNode)` | Hyperlink |
| `.image(ImageNode)` | Embedded image |
| `.autolink(AutolinkNode)` | Auto-detected URL or email |
| `.rawInline(RawInlineNode)` | Format-specific passthrough |
| `.hardBreak` | Forced line break (`<br>`) |
| `.softBreak` | Soft line break |
| `.strikethrough(StrikethroughNode)` | GFM strikethrough (`<del>`) |

## Usage

```swift
import DocumentAst

// Build a simple document programmatically
let doc = DocumentNode(children: [
    .heading(HeadingNode(level: 1, children: [
        .text(TextNode(value: "Hello, World!"))
    ])),
    .paragraph(ParagraphNode(children: [
        .text(TextNode(value: "This is a ")),
        .emphasis(EmphasisNode(children: [.text(TextNode(value: "great"))])),
        .text(TextNode(value: " document."))
    ]))
])

// Pass to a renderer
// let html = render(.document(doc))
```

## Role in the Stack

```
document-ast (Layer 0 — types only)
    ▲
    ├── document-ast-to-html  (HTML renderer)
    ├── commonmark-parser     (Markdown → AST)
    ├── gfm-parser            (GFM → AST)
    └── rst-parser            (RST → AST)
```

## Testing

```
swift test
```
