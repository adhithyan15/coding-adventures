# coding-adventures-gfm

GFM pipeline — thin re-export of `parse()` and `to_html()`.

## What is this?

This package is the single-import convenience wrapper for the full
GitHub Flavored Markdown → HTML pipeline.

```
Markdown
  ↓  parse()  [coding_adventures_gfm_parser]
DocumentNode  [coding_adventures_document_ast]
  ↓  to_html()  [coding_adventures_document_ast_to_html]
HTML string
```

## Quick Start

```python
from coding_adventures_gfm import parse, to_html

# Parse to AST
doc = parse("# Hello\n\nWorld\n")
doc["type"]  # "document"

# Render to HTML
html = to_html(doc)
# "<h1>Hello</h1>\n<p>World</p>\n"

# One-liner
html = to_html(parse("# Hello\n\nWorld\n"))
```

## Dependencies

This package simply re-exports from:
- `coding-adventures-gfm-parser` — the parser
- `coding-adventures-document-ast-to-html` — the HTML renderer
- `coding-adventures-document-ast` — the Document AST types

## Spec

This package implements the TE00 Document AST pipeline for GFM.
