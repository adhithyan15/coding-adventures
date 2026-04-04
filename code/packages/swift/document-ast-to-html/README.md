# document-ast-to-html (Swift)

HTML renderer for the Document AST.

## Overview

`document-ast-to-html` converts a `DocumentAst` node tree into an HTML string. It is the standard CommonMark HTML back-end in this project.

## Design

- Block nodes render with a trailing `\n`.
- Inline nodes render without a trailing newline.
- `rawBlock` / `rawInline` nodes with `format: "html"` are emitted verbatim.
- `rawBlock` / `rawInline` nodes with an unknown format are silently skipped.
- Tight lists suppress `<p>` wrappers around single-paragraph list items.
- `codeBlock` content is HTML-escaped.

## HTML Escaping

The characters `&`, `<`, `>`, and `"` are escaped in text content and attribute values. URLs are `&`-escaped in attributes but not otherwise re-encoded (the parser handles normalization).

## Usage

```swift
import DocumentAst
import DocumentAstToHtml

let doc = BlockNode.document(DocumentNode(children: [
    .paragraph(ParagraphNode(children: [.text(TextNode(value: "Hello"))]))
]))

let html = render(doc)  // → "<p>Hello</p>\n"
```

## Role in the Stack

```
document-ast (Layer 0)
    ▲
    └── document-ast-to-html (Layer 1 — HTML renderer)
            ▲
            └── commonmark (Layer 2 — convenience wrapper)
```

## Testing

```
swift test
```
