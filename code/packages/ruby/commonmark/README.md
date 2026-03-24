# coding_adventures_commonmark

CommonMark 0.31.2 pipeline convenience package — parse Markdown to HTML in one call.

## Overview

`coding_adventures_commonmark` is the thin top-level package that wires
`coding_adventures_commonmark_parser` and `coding_adventures_document_ast_to_html`
together into a minimal three-method API.

If you only need to go from Markdown to HTML, this is the only gem you need.
If you need access to the intermediate `DocumentNode` AST (to inspect the
document structure, drive a custom renderer, or produce non-HTML output), use
the constituent packages directly.

## Dependency Diagram

```
coding_adventures_document_ast           ← format-agnostic types (TE00)
         ↓ types                                 ↓ types
coding_adventures_commonmark_parser    coding_adventures_document_ast_to_html
  parse(markdown) → DocumentNode         to_html(doc) → String
         ↓ depends on both
coding_adventures_commonmark            ← you are here
  html = Commonmark.parse_to_html(markdown)
```

## Installation

```ruby
gem "coding_adventures_commonmark"
```

## Usage

### One-shot Markdown → HTML

```ruby
require "coding_adventures_commonmark"

html = CodingAdventures::Commonmark.parse_to_html("# Hello\n\nWorld\n")
# => "<h1>Hello</h1>\n<p>World</p>\n"
```

### Two-step (parse then render)

```ruby
doc  = CodingAdventures::Commonmark.parse("# Hello\n\nWorld\n")
doc.type                  # => "document"
doc.children[0].type      # => "heading"
doc.children[0].level     # => 1

html = CodingAdventures::Commonmark.to_html(doc)
# => "<h1>Hello</h1>\n<p>World</p>\n"
```

### Sanitized output (strip raw HTML)

```ruby
html = CodingAdventures::Commonmark.parse_to_html(markdown, sanitize: true)
```

## API

| Method | Description |
|--------|-------------|
| `Commonmark.parse(markdown)` | Parse Markdown string → `DocumentNode` |
| `Commonmark.to_html(doc, sanitize: false)` | Render `DocumentNode` → HTML string |
| `Commonmark.parse_to_html(markdown, sanitize: false)` | Parse + render in one call |

## Spec

Combines **TE01 — CommonMark Parser** and **TE02 — Document AST to HTML**, both
built on **TE00 — Document AST**.

## Requirements

- Ruby >= 3.2.0
- `coding_adventures_commonmark_parser` ~> 0.1
- `coding_adventures_document_ast_to_html` ~> 0.1
