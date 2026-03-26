# coding-adventures-gfm-parser

GitHub Flavored Markdown parser producing a Document AST.

## What is this?

This package parses Markdown source text into a `DocumentNode` — the format-agnostic
intermediate representation defined in `coding-adventures-document-ast`. The result
is ready for any back-end renderer (HTML, PDF, plain text, …).

The parse is two-phase:
- **Phase 1 — Block structure**: headings, lists, code blocks, blockquotes, …
- **Phase 2 — Inline content**: emphasis, links, images, code spans, …

## Quick Start

```python
from coding_adventures_gfm_parser import parse

doc = parse("# Hello\n\nWorld *with* emphasis.\n")
doc["type"]                  # "document"
doc["children"][0]["type"]   # "heading"
doc["children"][1]["type"]   # "paragraph"
```

## With the HTML renderer

```python
from coding_adventures_gfm_parser import parse
from coding_adventures_document_ast_to_html import to_html

html = to_html(parse("# Hello\n\nWorld\n"))
# → "<h1>Hello</h1>\n<p>World</p>\n"
```

## GFM Compliance

This parser passes all 652 examples from the [GFM 0.31.2 specification](https://spec.commonmark.org/0.31.2/).

Supported features:
- ATX and setext headings
- Thematic breaks
- Fenced and indented code blocks
- HTML blocks (all 7 types)
- Blockquotes with lazy continuation
- Ordered and unordered lists (tight and loose)
- Link reference definitions
- Inline: code spans, emphasis, strong, links, images, autolinks, raw HTML
- Backslash escapes and HTML entity decoding

## Spec

This package implements TE02 — GFM Parser (using the TE00 Document AST IR).
