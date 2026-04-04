# coding-adventures-asciidoc

Thin convenience wrapper that converts AsciiDoc source text to HTML in one call.

## How it fits in the stack

```
AsciiDoc source text
    ↓  asciidoc_parser.parse()
DocumentNode
    ↓  document_ast_to_html.to_html()
HTML string
```

This package (`coding_adventures_asciidoc`) chains those two steps into a single
`to_html(text)` call. It is the AsciiDoc counterpart to `coding_adventures_commonmark`.

## Installation

```bash
pip install coding-adventures-asciidoc
```

## Usage

```python
from coding_adventures_asciidoc import to_html

html = to_html("= Hello World\n\nThis is a *bold* paragraph.\n")
# "<h1>Hello World</h1>\n<p>This is a <strong>bold</strong> paragraph.</p>\n"
```

For direct access to the AST, use `coding_adventures_asciidoc_parser` directly:

```python
from coding_adventures_asciidoc_parser import parse
from coding_adventures_document_ast_to_html import to_html

doc = parse("= Title\n\nBody.\n")
html = to_html(doc)
```

## Spec

TE03 — AsciiDoc Parser
