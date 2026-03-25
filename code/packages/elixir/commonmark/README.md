# CodingAdventures.Commonmark

CommonMark 0.31.2 → HTML pipeline for Elixir.

A thin convenience wrapper that connects `CommonmarkParser` and `DocumentAstToHtml`
into a single `to_html/1` function. Use this package when you want to convert
CommonMark text to HTML in one call.

Spec: TE00 — Document AST

## How it fits in the stack

```
CodingAdventures.Commonmark (this package)
     │
     ├── CodingAdventures.CommonmarkParser  (CommonMark 0.31.2 → Document AST)
     │
     ├── CodingAdventures.DocumentAst       (format-agnostic IR)
     │
     └── CodingAdventures.DocumentAstToHtml (Document AST → HTML)
```

If you need access to the intermediate Document AST (e.g. to inspect the tree
or use a different renderer), use `CommonmarkParser.parse/1` directly instead.

## Usage

```elixir
alias CodingAdventures.Commonmark

Commonmark.to_html("# Hello\n\nWorld!\n")
# => "<h1>Hello</h1>\n<p>World!</p>\n"

Commonmark.to_html("**bold** and _italic_\n")
# => "<p><strong>bold</strong> and <em>italic</em></p>\n"

Commonmark.to_html("[link](http://example.com)\n")
# => "<p><a href=\"http://example.com\">link</a></p>\n"
```

## Running Tests

```sh
mix deps.get
mix test
```
