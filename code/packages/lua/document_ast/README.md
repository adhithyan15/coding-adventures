# coding-adventures-document-ast (Lua)

A typed Document AST (Abstract Syntax Tree) for structured documents, compatible with the CommonMark 0.31.2 specification. This package defines the data model shared by parsers and renderers — a clean boundary that lets you swap out either side without touching the other.

## Overview

The Document AST sits in the middle of the CommonMark pipeline:

```
Markdown source
      │
      ▼
  [commonmark_parser]  ← parses Markdown into Document AST nodes
      │
      ▼
  [Document AST]       ← this package — the shared data model
      │
      ▼
  [document_ast_to_html] ← renders Document AST to HTML
      │
      ▼
    HTML output
```

## Installation

```bash
luarocks install coding-adventures-document-ast
```

Or add to your `.rockspec` dependencies:

```lua
dependencies = {
    "coding-adventures-document-ast >= 0.1.0",
}
```

## Usage

```lua
local ast = require("coding_adventures.document_ast")

-- Build a simple document programmatically
local doc = ast.document({
    ast.heading(1, { ast.text("Hello, World!") }),
    ast.paragraph({
        ast.text("This is "),
        ast.strong({ ast.text("bold") }),
        ast.text(" text."),
    }),
    ast.thematic_break(),
    ast.code_block("lua", 'print("hello")\n'),
})
```

## Node Reference

### Block nodes

| Constructor | Description |
|---|---|
| `ast.document(children)` | Root node; `children` is a list of block nodes |
| `ast.heading(level, children)` | `<h1>`–`<h6>`; `level` is 1–6 |
| `ast.paragraph(children)` | `<p>`; `children` is a list of inline nodes |
| `ast.code_block(lang, literal)` | Fenced or indented code block; `lang` may be `nil` |
| `ast.blockquote(children)` | `<blockquote>`; `children` is a list of block nodes |
| `ast.list(ordered, start, tight, items)` | `<ul>` or `<ol>`; `tight` controls `<p>` wrapping |
| `ast.list_item(children)` | `<li>`; `children` is a list of block nodes |
| `ast.thematic_break()` | `<hr />` |
| `ast.raw_block(format, literal)` | Pass-through block (e.g. raw HTML) |

### Inline nodes

| Constructor | Description |
|---|---|
| `ast.text(literal)` | Plain text — HTML-escaped on render |
| `ast.emphasis(children)` | `<em>` |
| `ast.strong(children)` | `<strong>` |
| `ast.code_span(literal)` | `<code>` — content is HTML-escaped |
| `ast.link(url, title, children)` | `<a href>`; `title` may be `nil` |
| `ast.image(src, title, alt)` | `<img>`; `title` may be `nil` |
| `ast.autolink(value, is_email)` | `<a href>` with verbatim label |
| `ast.raw_inline(format, literal)` | Pass-through inline (e.g. raw HTML) |
| `ast.hard_break()` | `<br />` |
| `ast.soft_break()` | Newline (rendered as `\n`) |

## Node Shape

Every node is a plain Lua table with a `type` field:

```lua
{
    type     = "heading",
    level    = 2,
    children = { { type = "text", literal = "Subtitle" } },
}
```

Nodes are immutable by convention — constructors return new tables and nothing in the package mutates them.

## Testing

```bash
cd tests
busted . --verbose --pattern=test_
```

## Spec

See `code/specs/TE00-document-ast.md` for the full specification that this package implements.
