# coding-adventures-commonmark (Lua)

The CommonMark 0.31.2 pipeline for Lua: converts Markdown source to HTML in one call. This is a thin facade that wires together `coding-adventures-commonmark-parser` and `coding-adventures-document-ast-to-html`.

**All 652 CommonMark 0.31.2 specification examples pass.**

## Overview

```
Markdown source  →  [commonmark_parser]  →  Document AST  →  [document_ast_to_html]  →  HTML
```

If you only need Markdown-to-HTML, this is the package to use. If you need to inspect or transform the Document AST between parsing and rendering, depend on the individual packages instead.

## Installation

```bash
luarocks install coding-adventures-commonmark
```

Or add to your `.rockspec` dependencies:

```lua
dependencies = {
    "coding-adventures-commonmark >= 0.1.0",
}
```

## Usage

```lua
local commonmark = require("coding_adventures.commonmark")

local html = commonmark.to_html("# Hello\n\nThis is **CommonMark**.\n")
print(html)
-- <h1>Hello</h1>
-- <p>This is <strong>CommonMark</strong>.</p>
```

## API

### `commonmark.to_html(source, opts) -> string`

Parse a Markdown string and render it to HTML.

**Parameters**

- `source` — UTF-8 Markdown string.
- `opts` (optional table) — passed through to the renderer:
  - `opts.sanitize` — boolean (default `false`). When `true`, raw HTML blocks
    and raw HTML inlines are suppressed.

**Returns** an HTML string.

## Testing

```bash
cd tests
busted . --verbose --pattern=test_
```

The spec test file (`test_commonmark_spec.lua`) drives all 652 examples from the official CommonMark 0.31.2 specification JSON. All examples pass.

## Spec

See `code/specs/TE00-document-ast.md` for the Document AST specification and the [CommonMark spec](https://spec.commonmark.org/0.31.2/) for the full Markdown grammar.
