# coding_adventures_asciidoc_parser

AsciiDoc parser that converts AsciiDoc source text into the Document AST IR
defined in `coding_adventures_document_ast`.

## How it fits in the stack

```
AsciiDoc source text
    ↓  AsciidocParser.parse()      ← you are here
DocumentNode  (coding_adventures_document_ast)
    ↓  DocumentAstToHtml.to_html()
HTML string
```

The `coding_adventures_asciidoc` gem wraps both steps into a single `to_html(text)` call.

## Installation

Add to your Gemfile:

```ruby
gem "coding_adventures_asciidoc_parser"
```

## Usage

```ruby
require "coding_adventures_asciidoc_parser"

doc = CodingAdventures::AsciidocParser.parse("= Hello World\n\nThis is a *bold* paragraph.\n")
doc.type               # => "document"
doc.children[0].type   # => "heading"
doc.children[0].level  # => 1
```

## Supported constructs

**Block-level:**
- Headings: `= Level 1` through `====== Level 6`
- Paragraphs
- Thematic breaks: `'''`
- Fenced code blocks: `----` with optional `[source,lang]`
- Literal blocks: `....`
- Passthrough blocks: `++++` (→ RawBlockNode)
- Quote blocks: `____` (recursively parsed)
- Unordered lists: `* item`, `** nested`, etc.
- Ordered lists: `. item`, `.. nested`, etc.
- Line comments: `//` (skipped)

**Inline:**
- Strong: `*text*` and `**text**`
- Emphasis: `_text_` and `__text__`
- Code spans: `` `code` `` (verbatim)
- Links: `link:url[text]`
- Images: `image:url[alt]`
- Cross-references: `<<anchor,text>>` and `<<anchor>>`
- URLs: `https://url[text]` and bare autolinks
- Hard/soft breaks

## Spec

TE03 — AsciiDoc Parser
