# asciidoc

AsciiDoc → HTML pipeline for Go — parse AsciiDoc and render HTML in one call.

## Overview

This package is a thin convenience wrapper that combines `asciidoc-parser` and `document-ast-to-html` into a single easy-to-use API. For the full parsing details see the `asciidoc-parser` package.

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/asciidoc"

// Convert AsciiDoc to HTML
html := asciidoc.ToHtml("= Hello\n\nWorld *with* bold.\n")
// → "<h1>Hello</h1>\n<p>World <strong>with</strong> bold.</p>\n"

// Safe mode — strips raw HTML passthrough blocks (for untrusted input)
html := asciidoc.ToHtmlSafe(userInput)

// Parse only — get the Document AST
doc := asciidoc.Parse("= Title\n\nContent\n")
```

## Architecture

```
asciidoc (this package)
    ├── asciidoc-parser  (AsciiDoc text → DocumentNode)
    ├── document-ast     (shared IR types)
    └── document-ast-to-html  (DocumentNode → HTML)
```

## Spec

TE03 — AsciiDoc Parser
