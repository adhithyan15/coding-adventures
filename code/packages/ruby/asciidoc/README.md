# coding_adventures_asciidoc

Thin convenience wrapper that converts AsciiDoc source text to HTML in one call.

## How it fits in the stack

```
AsciiDoc source text
    ↓  AsciidocParser.parse()
DocumentNode
    ↓  DocumentAstToHtml.to_html()
HTML string
```

This gem (`coding_adventures_asciidoc`) chains those two steps into a single
`to_html(text)` call. It is the AsciiDoc counterpart to `coding_adventures_commonmark`.

## Installation

Add to your Gemfile:

```ruby
gem "coding_adventures_asciidoc"
```

## Usage

```ruby
require "coding_adventures_asciidoc"

html = CodingAdventures::Asciidoc.to_html("= Hello World\n\nThis is a *bold* paragraph.\n")
# => "<h1>Hello World</h1>\n<p>This is a <strong>bold</strong> paragraph.</p>\n"
```

For direct access to the AST, use the constituent packages:

```ruby
require "coding_adventures_asciidoc_parser"
require "coding_adventures_document_ast_to_html"

doc  = CodingAdventures::AsciidocParser.parse("= Title\n\nBody.\n")
html = CodingAdventures::DocumentAstToHtml.to_html(doc)
```

## Spec

TE03 — AsciiDoc Parser
