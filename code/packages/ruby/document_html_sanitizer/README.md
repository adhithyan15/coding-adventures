# coding_adventures-document_html_sanitizer

Regex-based HTML string sanitizer — no DOM dependency, no dependency on
`document-ast`. Part of the
[coding-adventures](https://github.com/adhithyan15/coding-adventures)
computing stack.

## Overview

The HTML sanitizer operates on an **opaque HTML string** with no knowledge of
how it was produced. It is a string → string transformation:

```
sanitize_html(html, policy) → safe_html
```

Use cases:
- HTML rendered by `document-ast-to-html`
- HTML from external APIs (CMS, third-party services)
- HTML pasted by users in rich-text editors

This is Stage 2 of the TE02 sanitization pipeline:

```
parse(markdown)          ← TE01 CommonMark Parser
      ↓
sanitize(doc, policy)    ← TE02 document-ast-sanitizer
      ↓
to_html(doc)             ← TE00 document-ast-to-html
      ↓
sanitize_html(html, pol) ← TE02 document-html-sanitizer  ← this gem
      ↓
final safe HTML
```

## Installation

```ruby
gem "coding_adventures_document_html_sanitizer"
```

## Usage

```ruby
require "coding_adventures/document_html_sanitizer"

include CodingAdventures

# Strict — untrusted HTML from external sources
safe = DocumentHtmlSanitizer.sanitize_html(html, DocumentHtmlSanitizer::HTML_STRICT)

# Relaxed — authenticated users / internal tools
safe = DocumentHtmlSanitizer.sanitize_html(html, DocumentHtmlSanitizer::HTML_RELAXED)

# Passthrough — fully trusted content (no sanitization)
safe = DocumentHtmlSanitizer.sanitize_html(html, DocumentHtmlSanitizer::HTML_PASSTHROUGH)

# Custom policy — derive from a preset and override specific fields
policy = DocumentHtmlSanitizer::HTML_STRICT.with(
  allowed_url_schemes: %w[http https mailto ftp],
  drop_comments: false
)
safe = DocumentHtmlSanitizer.sanitize_html(html, policy)
```

## Presets

### `HTML_STRICT` — untrusted HTML

Drops: `script`, `style`, `iframe`, `object`, `embed`, `applet`, `form`,
`input`, `button`, `select`, `textarea`, `noscript`, `meta`, `link`, `base`.

Strips: all `on*` event handlers, `srcdoc`, `formaction`.

URL schemes: `http`, `https`, `mailto` only.

Drops comments and sanitizes style attributes.

### `HTML_RELAXED` — semi-trusted HTML

Drops: `script`, `iframe`, `object`, `embed`, `applet`.

URL schemes: `http`, `https`, `mailto`, `ftp`.

Comments preserved. Style attributes sanitized.

### `HTML_PASSTHROUGH` — no sanitization

Everything passes through unchanged. Use only for fully trusted content.

## Policy Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `drop_elements` | `Array<String>` | (see presets) | Element names to drop including content |
| `drop_attributes` | `Array<String>` | `[]` | Additional attribute names to strip |
| `allowed_url_schemes` | `Array<String>` / `nil` | `%w[http https mailto]` | Allowed URL schemes for href/src |
| `drop_comments` | Boolean | `true` | Strip HTML comments |
| `sanitize_style_attributes` | Boolean | `true` | Strip dangerous CSS (expression(), unsafe url()) |

## XSS Attack Vectors Covered

- Script injection: `<script>alert(1)</script>`, `<SCRIPT>alert(1)</SCRIPT>`
- Event handlers: `onclick`, `onload`, `onerror`, `onfocus`, `onmouseover`, etc.
- JavaScript URLs: `href="javascript:alert(1)"`, `href="JAVASCRIPT:alert(1)"`
- Control character bypasses: `href="java\x00script:alert(1)"`
- Zero-width character bypasses: `href="\u200Bjavascript:alert(1)"`
- Data URIs: `href="data:text/html,..."`
- Blob URIs: `src="blob:https://..."`
- CSS expressions: `style="width:expression(alert(1))"`
- CSS url() injection: `style="background:url(javascript:alert(1))"`
- HTML comment hiding: `<!--<script>alert(1)</script>-->`
- srcdoc injection: `<iframe srcdoc="...">`
- formaction override: `<button formaction="https://evil.com">`

## Implementation Notes

The sanitizer uses regex/string operations (no DOM parser) for portability.
Three passes:

1. Strip HTML comments (if `drop_comments: true`)
2. Drop dangerous elements and their content (e.g. `<script>...content...</script>`)
3. Process surviving tags: strip dangerous attributes, sanitize URLs, sanitize styles

## Spec

[TE02 — Document Sanitization](../../../../specs/TE02-document-sanitization.md)
