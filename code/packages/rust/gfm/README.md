# commonmark

GFM 0.31.2 Markdown pipeline — convert Markdown to HTML in one call.

## What it does

This is the public-facing convenience crate that combines the `commonmark-parser` and `document-ast-to-html` crates into a simple two-function API:

```rust
use commonmark::markdown_to_html;

let html = markdown_to_html("# Hello\n\nWorld *with* emphasis.\n");
assert_eq!(html, "<h1>Hello</h1>\n<p>World <em>with</em> emphasis.</p>\n");
```

## API

### `markdown_to_html(markdown: &str) -> String`

Parse Markdown and render to HTML. Raw HTML is passed through verbatim (GFM-compliant). Use for **trusted** Markdown (documentation, static content).

### `markdown_to_html_safe(markdown: &str) -> String`

Same as `markdown_to_html` but strips all raw HTML from the output. Use for **untrusted** Markdown (user-supplied content, web applications).

```rust
use commonmark::markdown_to_html_safe;

// Attacker tries to inject a script tag:
let html = markdown_to_html_safe("<script>alert(1)</script>\n\n**bold**\n");
assert_eq!(html, "<p><strong>bold</strong></p>\n");
```

### Lower-level API

Users who want to work with the Document AST directly (for custom renderers, analysis, etc.) should use the constituent crates:

```rust
use commonmark_parser::parse;
use document_ast_to_html::{to_html, RenderOptions};

let doc = parse("# Hello\n\nWorld\n");
// doc is a DocumentNode — inspect, transform, or render it
let html = to_html(&doc, &Default::default());
```

## How it fits in the stack

```text
document-ast               ← format-agnostic types
      ↓ types                        ↓ types
commonmark-parser          document-ast-to-html
parse(markdown) → Doc      to_html(doc) → String
      ↓ depends on both
commonmark                 ← you are here
markdown_to_html(md)
```

## Compliance

652/652 (100%) of the GFM 0.31.2 specification examples pass.

## Version

0.1.0
