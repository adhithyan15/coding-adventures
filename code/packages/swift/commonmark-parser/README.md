# commonmark-parser (Swift)

CommonMark Markdown parser — converts Markdown text to a Document AST.

## Overview

`commonmark-parser` implements a CommonMark 0.31.2 subset parser. It converts Markdown text into the Document AST IR (from `document-ast`).

## Two-Phase Architecture

```
Markdown text
     │
     ▼  Phase 1: BlockParser
Block structure (headings, lists, code blocks, etc.)
     │
     ▼  Phase 2: InlineParser
Inline nodes (emphasis, links, code spans, etc.)
     │
     ▼
BlockNode.document(DocumentNode(...))
```

**Phase 1 — BlockParser**: Processes lines sequentially to identify structural elements. Raw inline content is stored as strings.

**Phase 2 — InlineParser**: Converts raw inline strings into `InlineNode` trees using recursive descent parsing.

## Supported Features

### Block Elements

| Feature | Syntax |
|---------|--------|
| ATX Headings | `# H1` through `###### H6` |
| Thematic Break | `---`, `***`, `___` |
| Fenced Code Block | ` ```lang ` / ` ``` ` |
| Indented Code Block | 4-space indent |
| Blockquote | `> text` |
| Unordered List | `- item`, `* item`, `+ item` |
| Ordered List | `1. item`, `1) item` |
| Paragraph | Blank-line separated text |
| Raw HTML Block | Lines starting with `<tag...` |

### Inline Elements

| Feature | Syntax |
|---------|--------|
| Emphasis | `*text*`, `_text_` |
| Strong | `**text**`, `__text__` |
| Code Span | `` `code` `` |
| Link | `[text](url "title")` |
| Image | `![alt](url "title")` |
| Autolink URL | `<https://url>` |
| Autolink Email | `<user@email>` |
| Hard Break | Two trailing spaces + newline |
| Soft Break | Single newline |
| Strikethrough | `~~text~~` (GFM) |
| Backslash Escape | `\*` → literal `*` |

## Usage

```swift
import CommonmarkParser

let doc = parse("# Hello\n\nWorld")
// → BlockNode.document(DocumentNode(children: [
//     .heading(HeadingNode(level: 1, children: [.text(TextNode(value: "Hello"))])),
//     .paragraph(ParagraphNode(children: [.text(TextNode(value: "World"))]))
//   ]))
```

## Role in the Stack

```
document-ast (Layer 0)
    ▲
    ├── commonmark-parser (Layer 1 — Markdown → AST)
    └── document-ast-to-html (Layer 1 — AST → HTML)
            ▲
            └── commonmark (Layer 2 — convenience wrapper)
```

## Testing

```
swift test
```
