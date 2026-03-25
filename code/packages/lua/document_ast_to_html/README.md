# coding-adventures-document-ast-to-html (Lua)

A Document AST → HTML renderer following CommonMark 0.31.2 rendering rules. Takes any Document AST node produced by `coding_adventures.document_ast` and serialises it to an HTML string.

## Overview

This renderer occupies the second stage of the CommonMark pipeline:

```
Document AST nodes
      │
      ▼
  [document_ast_to_html]  ← this package
      │
      ▼
    HTML string
```

Because this package only depends on the Document AST, it can be used with any parser that produces compatible AST nodes — not just the CommonMark parser.

## Installation

```bash
luarocks install coding-adventures-document-ast-to-html
```

Or add to your `.rockspec` dependencies:

```lua
dependencies = {
    "coding-adventures-document-ast-to-html >= 0.1.0",
}
```

## Usage

```lua
local html = require("coding_adventures.document_ast_to_html")
local ast  = require("coding_adventures.document_ast")

local doc = ast.document({
    ast.heading(1, { ast.text("Hello") }),
    ast.paragraph({ ast.text("World") }),
})

print(html.to_html(doc))
-- <h1>Hello</h1>
-- <p>World</p>
```

## API

### `html.to_html(node, opts) -> string`

Render a Document AST node (typically the root `document` node) to an HTML string.

**Parameters**

- `node` — any Document AST node.
- `opts` (optional table):
  - `opts.sanitize` — boolean (default `false`). When `true`, all raw HTML
    blocks and raw HTML inlines are suppressed rather than passed through.

**Returns** an HTML string. Block nodes are each followed by a newline. Inline
nodes produce no trailing newline.

## Rendering rules

| Node type | HTML output |
|---|---|
| `document` | Children concatenated |
| `heading` | `<h1>`…`<h6>` with newline |
| `paragraph` | `<p>…</p>\n`; omits `<p>` wrapper when inside a tight list |
| `code_block` | `<pre><code …>…</code></pre>\n`; `class="language-X"` when lang present |
| `blockquote` | `<blockquote>\n…</blockquote>\n` |
| `list` (unordered) | `<ul>\n…</ul>\n` |
| `list` (ordered, start=1) | `<ol>\n…</ol>\n` |
| `list` (ordered, start≠1) | `<ol start="N">\n…</ol>\n` |
| `list_item` | `<li>…</li>\n`; tight items have content inlined, loose items wrapped in `<p>` |
| `thematic_break` | `<hr />\n` |
| `raw_block` | Passed through verbatim if `format == "html"` and not sanitized |
| `text` | HTML-escaped literal |
| `emphasis` | `<em>…</em>` |
| `strong` | `<strong>…</strong>` |
| `code_span` | `<code>…</code>`; content HTML-escaped |
| `link` | `<a href="…" title="…">…</a>`; dangerous schemes (`javascript:`, `vbscript:`, `data:`) replaced with `""` |
| `image` | `<img src="…" alt="…" title="…" />`; dangerous schemes replaced with `""` |
| `autolink` | `<a href="…">…</a>`; email autolinks prepend `mailto:` |
| `raw_inline` | Passed through verbatim if `format == "html"` and not sanitized |
| `hard_break` | `<br />\n` |
| `soft_break` | `\n` |

## HTML escaping

All text content is escaped: `&` → `&amp;`, `<` → `&lt;`, `>` → `&gt;`, `"` → `&quot;`.

## URL sanitization

`href` and `src` attributes are checked for dangerous URI schemes regardless of
case and URL-encoding.  URLs matching `javascript:`, `vbscript:`, or `data:`
(after stripping whitespace) are replaced with an empty string.  This happens
unconditionally — it is not controlled by the `sanitize` option.

## Testing

```bash
cd tests
busted . --verbose --pattern=test_
```

52 tests covering every node type, HTML escaping, URL sanitization, tight/loose list rendering, the `sanitize` option, and edge cases. All tests pass with busted.

## Spec

See `code/specs/TE00-document-ast.md` for the Document AST specification this renderer targets.
