# asciidoc

AsciiDoc → HTML pipeline for the coding-adventures project.

A thin convenience wrapper that chains `asciidoc_parser` and
`document_ast_to_html` into a single `to_html/1` function.

## Architecture

```
AsciiDoc text
     │
     ▼
AsciidocParser.parse/1        (block + inline parsing)
     │
     ▼
CodingAdventures.DocumentAst  (IR: document node tree)
     │
     ▼
DocumentAstToHtml.render/1    (HTML back-end)
     │
     ▼
HTML string
```

The Document AST is the shared intermediate representation used by all parsers
and renderers in this project. This package is intentionally tiny — its value
is in connecting the two packages, not in doing any parsing or rendering itself.

## Usage

```elixir
CodingAdventures.Asciidoc.to_html("= Hello\n\nWorld\n")
# => "<h1>Hello</h1>\n<p>World</p>\n"

CodingAdventures.Asciidoc.to_html("*bold* and _italic_\n")
# => "<p><strong>bold</strong> and <em>italic</em></p>\n"
```

## Fitting into the stack

- **Depends on:** `document_ast`, `asciidoc_parser`, `document_ast_to_html`
- **Does not depend on:** any external libraries

## Testing

```
mix deps.get && mix test --cover
```
