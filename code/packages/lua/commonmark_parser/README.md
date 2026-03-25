# coding-adventures-commonmark-parser (Lua)

A CommonMark 0.31.2 compliant Markdown parser written in pure Lua. Produces a Document AST that can be rendered to HTML (or any other output format) by a separate renderer package.

**All 652 CommonMark 0.31.2 specification examples pass.**

## Overview

This parser occupies the first stage of the CommonMark pipeline:

```
Markdown source
      │
      ▼
  [commonmark_parser]  ← this package
      │
      ▼
  Document AST nodes
```

Parsing follows the two-phase algorithm described in the CommonMark spec:

1. **Block structure** — scan the source line by line to identify block boundaries (paragraphs, headings, fenced code blocks, lists, blockquotes, HTML blocks, link reference definitions, etc.).
2. **Inline content** — scan each block's raw text for inline constructs (emphasis, strong, links, images, code spans, autolinks, raw HTML, hard/soft breaks, entity references, backslash escapes).

## Installation

```bash
luarocks install coding-adventures-commonmark-parser
```

Or add to your `.rockspec` dependencies:

```lua
dependencies = {
    "coding-adventures-commonmark-parser >= 0.1.0",
}
```

## Usage

```lua
local parser = require("coding_adventures.commonmark_parser")
local ast    = require("coding_adventures.document_ast")

local source = [[
# Hello

This is a **CommonMark** document.

- item one
- item two
]]

local doc = parser.parse(source)
-- doc is a Document AST node with type == "document"
-- doc.children contains the block-level nodes
```

## API

### `parser.parse(source) -> document_node`

Parse a Markdown string and return the root `document` AST node.

- `source` — UTF-8 Markdown string.  Line endings may be `\n` or `\r\n`; they are normalised to `\n` before parsing.
- Returns a `document` node as defined by `coding_adventures.document_ast`.

## Supported CommonMark features

| Feature | Notes |
|---|---|
| ATX headings (`# H1` … `###### H6`) | All levels |
| Setext headings | `===` and `---` underlines |
| Indented code blocks | 4-space indent |
| Fenced code blocks | `` ``` `` and `~~~` with info string |
| Blockquotes | Nested blockquotes supported |
| Ordered and unordered lists | Tight and loose; ordered lists with any start number |
| Thematic breaks | `---`, `***`, `___` |
| Link reference definitions | Full title support including multi-line titles |
| ATX / setext HTML blocks | All 7 CommonMark HTML block types |
| Inline code spans | Backtick stripping and whitespace normalisation |
| Emphasis and strong | Full left-/right-flanking delimiter run algorithm |
| Links and images | Inline and reference forms; title attributes |
| Autolinks | URL and email autolinks (`<...>`) |
| Raw inline HTML | Passed through verbatim |
| Hard and soft line breaks | `\  ` and single newline |
| Backslash escapes | All ASCII punctuation |
| Entity references | Named, decimal, and hexadecimal; digit-count limits enforced |
| Unicode | UTF-8 aware delimiter flanking; Unicode case folding for link labels |

## Testing

```bash
cd tests
busted . --verbose --pattern=test_
```

The `test_commonmark_spec.lua` file drives all 652 examples from the official CommonMark 0.31.2 spec JSON. The `test_commonmark_parser.lua` file provides 77 additional unit tests targeting specific parser behaviours.

## Spec

See `code/specs/TE00-document-ast.md` for the Document AST specification and the [CommonMark spec](https://spec.commonmark.org/0.31.2/) for the full Markdown grammar this parser implements.
