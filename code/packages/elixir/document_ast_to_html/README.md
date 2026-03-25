# CodingAdventures.DocumentAstToHtml

Document AST → HTML renderer for Elixir.

Converts a `CodingAdventures.DocumentAst` node tree into a CommonMark-compliant
HTML string. This is the standard HTML back-end for the Document AST pipeline.

Spec: TE00 — Document AST

## How it fits in the stack

```
CodingAdventures.DocumentAst  (any front-end produces this)
     │
     ▼
DocumentAstToHtml.render/1
     │
     ▼
HTML string
```

## Usage

```elixir
alias CodingAdventures.DocumentAst
alias CodingAdventures.DocumentAstToHtml

doc = DocumentAst.document([
  DocumentAst.paragraph([DocumentAst.text("Hello, world!")])
])

DocumentAstToHtml.render(doc)
# => "<p>Hello, world!</p>\n"
```

With the CommonMark parser:

```elixir
alias CodingAdventures.CommonmarkParser
alias CodingAdventures.DocumentAstToHtml

{doc, _refs} = CommonmarkParser.parse("# Hello\n\nWorld!\n")
html = DocumentAstToHtml.render(doc)
# => "<h1>Hello</h1>\n<p>World!</p>\n"
```

## Design

### Block Rendering

Every block node renders with a trailing newline. Tight lists suppress `<p>`
wrappers around list item paragraph content (per CommonMark §5.3).

### Inline Rendering

Inline nodes render without trailing newlines. `<br />` is used for hard breaks;
`\n` for soft breaks.

### HTML Escaping

The characters `&`, `<`, `>`, and `"` are escaped in text content and attribute
values. URLs in `href` and `src` are NOT re-encoded (the parser normalizes them),
except that `&` is escaped to `&amp;` for HTML attribute validity.

### Raw Content

`raw_block` and `raw_inline` nodes with `format: "html"` are emitted verbatim.
Nodes with unknown format are silently skipped.

## Running Tests

```sh
mix deps.get
mix test
```
