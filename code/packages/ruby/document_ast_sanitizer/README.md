# coding_adventures-document_ast_sanitizer

Policy-driven AST sanitizer for the Document AST pipeline. Part of the
[coding-adventures](https://github.com/adhithyan15/coding-adventures)
computing stack.

## Overview

This gem slots between the CommonMark parser and the HTML renderer in the
document processing pipeline:

```
parse(markdown)          ← TE01 CommonMark Parser
      ↓
sanitize(doc, policy)    ← TE02 document-ast-sanitizer  ← this gem
      ↓
to_html(doc)             ← TE00 document-ast-to-html
      ↓
sanitize_html(html, pol) ← TE02 document-html-sanitizer
      ↓
final safe HTML
```

Sanitization is a **separate pipeline concern** from parsing and rendering.
The parser's job is to faithfully parse Markdown into an AST. The renderer's
job is to faithfully convert the AST to HTML. Neither component should decide
what content is "safe" — that is the sanitizer's job.

## Installation

Add to your Gemfile (using the path reference for monorepo use):

```ruby
gem "coding_adventures_document_ast_sanitizer", path: "../document_ast_sanitizer"
```

Or from RubyGems once published:

```ruby
gem "coding_adventures_document_ast_sanitizer"
```

## Usage

```ruby
require "coding_adventures/document_ast_sanitizer"
require "coding_adventures/document_ast_to_html"

include CodingAdventures

# --- Stage 1 only: AST sanitization (recommended for Markdown pipelines) ---

# Strict mode — for user-generated content (comments, forum posts)
safe_doc = DocumentAstSanitizer.sanitize(doc, DocumentAstSanitizer::STRICT)
html = DocumentAstToHtml.to_html(safe_doc)

# Relaxed mode — for authenticated users, internal wikis
safe_doc = DocumentAstSanitizer.sanitize(doc, DocumentAstSanitizer::RELAXED)

# Passthrough — for fully trusted content (documentation, static sites)
safe_doc = DocumentAstSanitizer.sanitize(doc, DocumentAstSanitizer::PASSTHROUGH)

# Custom policy — derive from a preset and override specific fields
policy = DocumentAstSanitizer::RELAXED.with(
  min_heading_level: 2,           # reserve h1 for page title
  allowed_url_schemes: %w[http https]
)
safe_doc = DocumentAstSanitizer.sanitize(doc, policy)
```

## Presets

### `STRICT` — user-generated content

- Drops all raw HTML/format passthrough
- Allows only `http`, `https`, `mailto` URLs
- Converts images to alt text (no tracking pixels)
- Clamps headings to h2–h6 (h1 reserved for page title)

### `RELAXED` — semi-trusted content

- Allows HTML raw blocks (but not LaTeX or others)
- Allows `http`, `https`, `mailto`, `ftp` URLs
- Images pass through unchanged
- All heading levels allowed

### `PASSTHROUGH` — fully trusted content

- No sanitization. Equivalent to not calling `sanitize()` at all.

## Policy Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `allow_raw_block_formats` | `"drop-all"` / `"passthrough"` / `Array<String>` | `"passthrough"` | Which RawBlockNode formats to keep |
| `allow_raw_inline_formats` | same | `"passthrough"` | Which RawInlineNode formats to keep |
| `allowed_url_schemes` | `Array<String>` / `nil` | `%w[http https mailto ftp]` | Allowed URL schemes (nil = any) |
| `drop_links` | Boolean | `false` | Promote link children, remove anchor wrapper |
| `drop_images` | Boolean | `false` | Remove ImageNode entirely |
| `transform_image_to_text` | Boolean | `false` | Replace ImageNode with alt TextNode |
| `max_heading_level` | 1–6 / `"drop"` | `6` | Clamp or drop deep headings |
| `min_heading_level` | 1–6 | `1` | Clamp shallow headings upward |
| `drop_blockquotes` | Boolean | `false` | Remove BlockquoteNode |
| `drop_code_blocks` | Boolean | `false` | Remove CodeBlockNode |
| `transform_code_span_to_text` | Boolean | `false` | Convert CodeSpanNode to TextNode |

## XSS Protection

The sanitizer blocks these attack vectors:

- `javascript:alert(1)` — scheme blocked by `allowed_url_schemes`
- `java\x00script:alert(1)` — C0 control char stripped before scheme detection
- `\u200Bjavascript:alert(1)` — zero-width space stripped before scheme detection
- `data:text/html,...` — scheme blocked
- `blob:https://...` — scheme blocked
- Raw HTML injection via `RawBlockNode` — dropped by `allow_raw_block_formats: "drop-all"`

## Pipeline Integration

```ruby
# Two-stage: belt and suspenders
require "coding_adventures/document_ast_sanitizer"
require "coding_adventures/document_html_sanitizer"
require "coding_adventures/document_ast_to_html"

safe_html = DocumentHtmlSanitizer.sanitize_html(
  DocumentAstToHtml.to_html(
    DocumentAstSanitizer.sanitize(doc, DocumentAstSanitizer::STRICT)
  ),
  DocumentHtmlSanitizer::HTML_STRICT
)
```

## Spec

[TE02 — Document Sanitization](../../../../specs/TE02-document-sanitization.md)
