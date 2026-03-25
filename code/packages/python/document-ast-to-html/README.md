# coding-adventures-document-ast-to-html

Document AST → HTML renderer (back-end for the Document AST pipeline).

## What is this?

This package converts a `DocumentNode` (from `coding-adventures-document-ast`)
into an HTML string. It is one of the back-ends in the Document AST pipeline.

```
Markdown ──► commonmark-parser ──► DocumentNode ──► document-ast-to-html ──► HTML
```

## Quick Start

```python
from coding_adventures_commonmark_parser import parse
from coding_adventures_document_ast_to_html import to_html

html = to_html(parse("# Hello\n\nWorld\n"))
# → "<h1>Hello</h1>\n<p>World</p>\n"
```

## Security

⚠️ **Raw HTML passthrough is enabled by default** (required for CommonMark spec compliance).
If you render **untrusted** Markdown (user content, third-party data), use:

```python
from coding_adventures_document_ast_to_html import to_html, RenderOptions

html = to_html(parse(user_markdown), RenderOptions(sanitize=True))
```

This strips all raw HTML blocks/inlines from the output.

## Spec

This package implements the HTML rendering backend for the TE00 Document AST.
