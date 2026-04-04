# coding-adventures-asciidoc-parser

AsciiDoc parser that converts AsciiDoc source text into the Document AST IR
defined in `coding-adventures-document-ast`.

## How it fits in the stack

```
AsciiDoc source text
    ↓  asciidoc_parser.parse()      ← you are here
DocumentNode  (coding_adventures_document_ast)
    ↓  document_ast_to_html.to_html()
HTML string
```

The `coding-adventures-asciidoc` package wraps both steps into a single
`to_html(text)` call.

## Installation

```bash
pip install coding-adventures-asciidoc-parser
```

## Usage

```python
from coding_adventures_asciidoc_parser import parse

# Parse AsciiDoc into a Document AST
doc = parse("= Hello World\n\nThis is a *bold* paragraph.\n")

doc["type"]                  # "document"
doc["children"][0]["type"]   # "heading"
doc["children"][0]["level"]  # 1
doc["children"][1]["type"]   # "paragraph"
```

## Supported AsciiDoc constructs

**Block-level:**
- Headings: `= Level 1` through `====== Level 6`
- Paragraphs (any non-special text)
- Thematic breaks: `'''` (three or more single-quotes)
- Fenced code blocks: `----` with optional `[source,lang]` attribute
- Literal blocks: `....`
- Passthrough blocks: `++++` (emits `RawBlockNode { format: "html" }`)
- Quote blocks: `____` (recursively parsed → `BlockquoteNode`)
- Unordered lists: `* item`, `** nested`, etc.
- Ordered lists: `. item`, `.. nested`, etc.
- Line comments: `// comment` (skipped)

**Inline:**
- Strong (bold): `*text*` and `**text**`
- Emphasis (italic): `_text_` and `__text__`
- Code spans: `` `code` `` (verbatim)
- Links: `link:url[text]`
- Images: `image:url[alt]`
- Cross-references: `<<anchor,text>>` and `<<anchor>>`
- URLs: `https://url[text]` and bare `https://...` (autolink)
- Hard breaks: two trailing spaces or `\` before newline
- Soft breaks: bare newlines within paragraphs

## Spec

TE03 — AsciiDoc Parser
