# CodingAdventures.DocumentAstSanitizer

Policy-driven Document AST sanitizer for Elixir. Slots between the CommonMark
parser and the HTML renderer to strip dangerous content from structured document
trees before they are rendered.

## Overview

```
parse(markdown)        ← CodingAdventures.CommonmarkParser
      ↓
sanitize(doc, policy)  ← CodingAdventures.DocumentAstSanitizer  ← this package
      ↓
render(doc)            ← CodingAdventures.DocumentAstToHtml
```

The sanitizer performs a **pure, recursive tree transformation**: it walks the
Document AST and applies a caller-defined policy to each node, deciding whether
to keep, transform, clamp, or drop it. The input document is never mutated.

## Quick Start

```elixir
alias CodingAdventures.DocumentAstSanitizer
alias CodingAdventures.DocumentAstSanitizer.Policy

# User-generated content — strict mode (most restrictive)
safe_doc = DocumentAstSanitizer.sanitize(parsed_doc, Policy.strict())

# Internal wiki — relaxed mode
wiki_doc = DocumentAstSanitizer.sanitize(parsed_doc, Policy.relaxed())

# Fully trusted documentation — no sanitization
docs_doc = DocumentAstSanitizer.sanitize(parsed_doc, Policy.passthrough())

# Custom policy: reserve h1 for page title, allow only https URLs
custom = %Policy{Policy.strict() | min_heading_level: 2,
                                   allowed_url_schemes: ["https"]}
result = DocumentAstSanitizer.sanitize(parsed_doc, custom)
```

## Policy Presets

| Preset         | Audience                             | Raw blocks | URL schemes            | Images                  |
|----------------|--------------------------------------|------------|------------------------|-------------------------|
| `strict/0`     | Anonymous users, comments, chat      | Drop all   | http, https, mailto    | Converted to alt text   |
| `relaxed/0`    | Authenticated users, internal wikis  | HTML only  | http, https, mailto, ftp | Passed through        |
| `passthrough/0`| Fully trusted, static sites          | All pass   | All pass (nil)         | Passed through          |

## Policy Options

```elixir
%CodingAdventures.DocumentAstSanitizer.Policy{
  # Raw node handling
  allow_raw_block_formats: :drop_all | :passthrough | ["html", ...],
  allow_raw_inline_formats: :drop_all | :passthrough | ["html", ...],
  # URL scheme allowlist (nil = allow all; relative URLs always pass)
  allowed_url_schemes: ["http", "https", "mailto"] | nil,
  # Node type policy
  drop_links: false,
  drop_images: false,
  transform_image_to_text: true,   # ImageNode → TextNode{alt}
  max_heading_level: 6,            # or :drop to remove all headings
  min_heading_level: 1,            # e.g. 2 reserves h1 for page title
  drop_blockquotes: false,
  drop_code_blocks: false,
  transform_code_span_to_text: false
}
```

## XSS Protection

The URL sanitizer strips C0 control characters and zero-width characters before
scheme extraction, blocking bypasses like `java\x00script:alert(1)`.

## Pipeline Integration

```elixir
alias CodingAdventures.CommonmarkParser
alias CodingAdventures.DocumentAstSanitizer
alias CodingAdventures.DocumentAstSanitizer.Policy
alias CodingAdventures.DocumentAstToHtml

# Single-stage: AST sanitization only (recommended)
html = DocumentAstToHtml.render(
  DocumentAstSanitizer.sanitize(CommonmarkParser.parse(user_markdown), Policy.strict())
)

# Two-stage: belt and suspenders
alias CodingAdventures.DocumentHtmlSanitizer
alias CodingAdventures.DocumentHtmlSanitizer.Policy, as: HtmlPolicy

safe_html = DocumentHtmlSanitizer.sanitize_html(
  DocumentAstToHtml.render(
    DocumentAstSanitizer.sanitize(CommonmarkParser.parse(user_markdown), Policy.strict())
  ),
  HtmlPolicy.html_strict()
)
```

## Dependencies

- `coding_adventures_document_ast` — Document AST node types

## Spec

Implements [TE02 — Document Sanitization](../../../specs/TE02-document-sanitization.md), Stage 1.
