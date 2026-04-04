# asciidoc

AsciiDoc → HTML pipeline for Rust — parse AsciiDoc and render HTML in one call.

## Overview

Thin convenience crate combining `asciidoc-parser` and `document-ast-to-html`.

## Usage

```rust
use asciidoc::asciidoc_to_html;

let html = asciidoc_to_html("= Hello\n\nWorld *with* bold.\n");
// html contains "<h1>Hello</h1>" and "<strong>with</strong>"

// Safe mode — strips raw HTML passthrough blocks (for untrusted input)
let html = asciidoc::asciidoc_to_html_safe(user_content);

// Parse only — get the Document AST
let doc = asciidoc::parse("= Title\n\nContent\n");
```

## Spec

TE03 — AsciiDoc Parser
