# asciidoc-parser

AsciiDoc parser for Go — converts AsciiDoc source text into a Document AST.

## Overview

This package implements a two-phase AsciiDoc parser:

1. **Block parser** (`block_parser.go`) — state machine that identifies headings, paragraphs, lists, code blocks, quote blocks, and thematic breaks.
2. **Inline parser** (`inline_parser.go`) — processes inline markup: bold, italic, links, images, code spans, and cross-references.

The output is a `DocumentNode` from the `document-ast` package — the same format-agnostic IR used by the Markdown parsers. Any back-end renderer that works with the Document AST (such as `document-ast-to-html`) can render AsciiDoc output without modification.

## AsciiDoc vs Markdown

Key differences:

| Construct | Markdown | AsciiDoc |
|---|---|---|
| Heading level 1 | `# Title` | `= Title` |
| Bold / Strong | `**bold**` | `*bold*` |
| Italic / Emphasis | `*italic*` | `_italic_` |
| Code block | ` ``` ` | `----` (with optional `[source,lang]`) |
| Thematic break | `---` | `'''` |
| Link | `[text](url)` | `link:url[text]` |
| Image | `![alt](url)` | `image:url[alt]` |
| Blockquote | `> text` | `____` block |

**Important:** In AsciiDoc, `*text*` produces `<strong>` (not `<em>`). This is the opposite of Markdown.

## Usage

```go
import parser "github.com/adhithyan15/coding-adventures/code/packages/go/asciidoc-parser"

doc := parser.Parse("= Hello\n\nWorld *with* bold text.\n")
// doc.Children[0] is HeadingNode{Level: 1}
// doc.Children[1] is ParagraphNode with StrongNode inside
```

## Supported AsciiDoc Constructs

### Block-level
- Headings: `= Title` through `====== Deep`
- Paragraphs (multi-line)
- Thematic break: `'''`
- Fenced code blocks: `----` with optional `[source,lang]`
- Literal blocks: `....`
- Passthrough blocks: `++++` (raw HTML)
- Quote blocks: `____` (recursively parsed)
- Unordered lists: `* item`, `** nested`
- Ordered lists: `. item`, `.. nested`
- Comments: `// text` (skipped)

### Inline
- Strong: `*text*` and `**text**` (unconstrained)
- Emphasis: `_text_` and `__text__` (unconstrained)
- Code span: `` `code` ``
- Links: `link:url[text]` and `https://url[text]`
- Autolinks: bare `https://url`
- Images: `image:url[alt]`
- Cross-references: `<<anchor,text>>` and `<<anchor>>`
- Hard breaks: two trailing spaces or backslash before newline
- Soft breaks: plain newline

## Architecture

```
asciidoc-parser
    └── depends on: document-ast
```

Part of the document processing pipeline:

```
AsciiDoc text
    │
    ▼
asciidoc-parser (this package)
    │
    ▼
DocumentNode (document-ast)
    │
    ▼
document-ast-to-html
    │
    ▼
HTML string
```

Or use the `asciidoc` convenience package for a one-call `ToHtml(text)` API.

## Spec

TE03 — AsciiDoc Parser
