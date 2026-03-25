# CodingAdventures.DocumentAst

Format-agnostic intermediate representation (IR) for structured documents.

The Document AST is the "LLVM IR of documents" — a stable, typed tree that
every front-end parser produces and every back-end renderer consumes. With a
shared IR, N front-ends × M back-ends requires only N + M implementations
instead of N × M.

```
Markdown ────────────────────────────────► HTML
reStructuredText ────► Document AST ────► PDF
HTML ────────────────────────────────────► Plain text
DOCX ────────────────────────────────────► DOCX
```

Spec: TE00 — Document AST

## How it fits in the stack

This is a types-only package — it provides constructor functions and type
specs for the Document AST nodes. It has no dependencies and no runtime logic.
The parser (`commonmark_parser`) and renderer (`document_ast_to_html`) depend
on this package.

## Usage

```elixir
alias CodingAdventures.DocumentAst

doc = DocumentAst.document([
  DocumentAst.heading(1, [DocumentAst.text("Hello")]),
  DocumentAst.paragraph([
    DocumentAst.text("World with "),
    DocumentAst.emphasis([DocumentAst.text("emphasis")])
  ])
])
```

## Node Types

### Block Nodes

- `:document` — root node, contains block children
- `:heading` — level 1-6, contains inline children
- `:paragraph` — contains inline children
- `:code_block` — raw code with optional language hint
- `:blockquote` — contains block children
- `:list` — ordered or unordered, contains list items
- `:list_item` — contains block children
- `:thematic_break` — horizontal rule
- `:raw_block` — verbatim passthrough with format tag

### Inline Nodes

- `:text` — plain text (HTML entities decoded)
- `:emphasis` — `<em>` equivalent
- `:strong` — `<strong>` equivalent
- `:code_span` — inline code
- `:link` — hyperlink with resolved destination
- `:image` — embedded image
- `:autolink` — URL or email address
- `:raw_inline` — verbatim passthrough with format tag
- `:hard_break` — forced line break (`<br />`)
- `:soft_break` — soft line break (newline in source)
