# commonmark (Swift)

CommonMark → HTML convenience wrapper.

## Overview

`commonmark` is a thin wrapper that chains `commonmark-parser` and `document-ast-to-html` into a single `toHtml(_:)` function. It provides the simplest possible API for converting Markdown to HTML.

## Architecture

```
CommonMark text
     │
     ▼
CommonmarkParser.parse(_:)    → BlockNode.document(...)
     │
     ▼
DocumentAstToHtml.render(_:)  → HTML string
```

The Document AST is the shared intermediate representation (IR) that decouples the parser from the renderer. This package simply connects the two.

## Usage

```swift
import Commonmark

toHtml("# Hello")
// → "<h1>Hello</h1>\n"

toHtml("**bold** and *italic*")
// → "<p><strong>bold</strong> and <em>italic</em></p>\n"

toHtml("> blockquote")
// → "<blockquote>\n<p>blockquote</p>\n</blockquote>\n"
```

## Role in the Stack

```
document-ast (Layer 0 — types)
    ▲
    ├── commonmark-parser    (Layer 1 — Markdown → AST)
    └── document-ast-to-html (Layer 1 — AST → HTML)
             ▲
             └── commonmark  (Layer 2 — pipeline wrapper)
```

## Testing

```
swift test
```
