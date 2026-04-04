# asciidoc-parser

AsciiDoc parser for Rust — converts AsciiDoc source text into a Document AST.

## Overview

Two-phase AsciiDoc parser:

1. **Block parser** (`block_parser.rs`) — state machine identifying headings, paragraphs, lists, code blocks, quote blocks, thematic breaks.
2. **Inline parser** (`inline_parser.rs`) — processes bold, italic, links, images, code spans, and cross-references.

Output is a `DocumentNode` from the `document-ast` crate — the same format-agnostic IR shared by the Markdown parsers.

## AsciiDoc vs Markdown

| Construct | Markdown | AsciiDoc |
|---|---|---|
| Bold | `**bold**` | `*bold*` |
| Italic | `*italic*` | `_italic_` |
| Heading level 1 | `# Title` | `= Title` |
| Code block | ` ``` ` | `----` |
| Thematic break | `---` | `'''` |
| Link | `[text](url)` | `link:url[text]` |
| Image | `![alt](url)` | `image:url[alt]` |

**Important:** `*text*` in AsciiDoc = `<strong>` (bold), not `<em>`.

## Usage

```rust
use asciidoc_parser::parse;

let doc = parse("= Hello\n\nWorld *with* bold.\n");
assert_eq!(doc.children.len(), 2);
```

## Spec

TE03 — AsciiDoc Parser
