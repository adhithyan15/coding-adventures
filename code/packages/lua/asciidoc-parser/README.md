# coding-adventures-asciidoc-parser

A pure-Lua AsciiDoc parser that converts AsciiDoc source text into a
Document AST (array of block-node tables), compatible with the
`coding_adventures.document_ast_to_html` renderer.

## What is AsciiDoc?

AsciiDoc is a lightweight markup language designed for technical writing.
It is richer than Markdown and is the format used by the Asciidoctor toolchain
to produce books, documentation sites, and man pages. Key differences from
CommonMark Markdown:

- `*bold*` → **strong** (not emphasis — that is `_italic_`)
- Headings use `=` sigils: `= H1`, `== H2`, …, `====== H6`
- Code blocks delimited by four dashes: `----`
- Literal blocks delimited by four dots: `....`
- Passthrough blocks delimited by four plus signs: `++++`
- Quote blocks delimited by four underscores: `____`
- Source language annotation: `[source,python]` before a `----` block
- Links: `link:https://example.com[click here]`
- Images: `image:photo.png[alt text]`
- Cross-references: `<<anchor-id,Link Text>>`

## Architecture

```
AsciiDoc text
    ↓
block_parser.parse_blocks(text) → {block, block, …}
    ↓                              (each block is a Lua table)
inline_parser.parse_inline(text) → {inline_node, …}
    ↓
init.parse(text) → document node  { type="document", children={…} }
```

The parser is split into two modules following the same pattern as the
`commonmark_parser` package:

- **`block_parser`** — state machine that walks lines and emits block nodes.
- **`inline_parser`** — left-to-right scanner that emits inline nodes.

Nodes are plain Lua tables: `{type = "heading", level = 1, children = {…}}`.

## Usage

```lua
local parser = require("coding_adventures.asciidoc_parser")

local doc = parser.parse([[
= Getting Started

This is a *bold* introduction.

== Installation

[source,bash]
----
luarocks install coding-adventures-asciidoc
----
]])

print(doc.type)              -- "document"
print(doc.children[1].type)  -- "heading"
print(doc.children[1].level) -- 1
```

## Supported AsciiDoc features

### Block elements
| Feature            | Syntax                     |
|--------------------|---------------------------|
| Heading 1–6        | `= H1` … `====== H6`      |
| Thematic break     | `'''` (3+ apostrophes)    |
| Paragraph          | consecutive text lines     |
| Code block         | `----` … `----`           |
| Literal block      | `....` … `....`           |
| Passthrough block  | `++++` … `++++`           |
| Quote / sidebar    | `____` … `____`           |
| Unordered list     | `* item` / `** nested`    |
| Ordered list       | `. item` / `.. nested`    |
| Source annotation  | `[source,lang]`           |
| Comment            | `// text` (skipped)       |

### Inline elements
| Feature             | Syntax                      |
|---------------------|-----------------------------|
| Strong (bold)       | `*text*` or `**text**`     |
| Emphasis (italic)   | `_text_` or `__text__`     |
| Inline code         | `` `code` ``               |
| Link                | `link:url[text]`           |
| Image               | `image:url[alt]`           |
| Cross-reference     | `<<anchor,text>>`          |
| Bare URL            | `https://…` or `http://…` |
| Hard break          | `+` at end of line         |

## Running tests

```
cd tests && busted . --verbose --pattern=test_
```

## License

MIT
