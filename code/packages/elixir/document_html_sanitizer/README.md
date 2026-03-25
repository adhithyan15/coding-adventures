# CodingAdventures.DocumentHtmlSanitizer

Regex-based HTML string sanitizer for Elixir. Strips dangerous elements and
attributes from opaque HTML strings. No dependencies — string in, string out.

## Overview

```
toHtml(doc)            ← CodingAdventures.DocumentAstToHtml
      ↓
sanitize_html(html, p) ← CodingAdventures.DocumentHtmlSanitizer  ← this package
      ↓
final output
```

The HTML sanitizer operates on rendered HTML strings with no knowledge of how
they were produced. It is useful as:

- A **belt-and-suspenders** layer after AST sanitization
- A sanitizer for **HTML from external sources** (CMS APIs, third-party services)
- A sanitizer for **user-pasted HTML** in rich-text editors

## Quick Start

```elixir
alias CodingAdventures.DocumentHtmlSanitizer
alias CodingAdventures.DocumentHtmlSanitizer.Policy

# Untrusted HTML from a third-party API
safe = DocumentHtmlSanitizer.sanitize_html(cms_html, Policy.html_strict())

# HTML from authenticated users
safe = DocumentHtmlSanitizer.sanitize_html(user_html, Policy.html_relaxed())

# No sanitization — trusted static-site output
safe = DocumentHtmlSanitizer.sanitize_html(trusted_html, Policy.html_passthrough())

# Custom: keep all elements but strip all URLs
custom = %Policy{Policy.html_passthrough() |
  allowed_url_schemes: ["https"],
  drop_comments: true}
```

## What Is Sanitized

**Always stripped (baseline security, not configurable):**
- `on*` event handler attributes (`onclick`, `onload`, `onerror`, etc.)
- `srcdoc` attribute (inline HTML in iframes)
- `formaction` attribute (URL override for form submission)

**Configurable via policy:**
- **Dropped elements** (including all content): `<script>`, `<style>`,
  `<iframe>`, `<object>`, `<embed>`, `<applet>`, `<form>`, `<input>`,
  `<button>`, `<select>`, `<textarea>`, `<noscript>`, `<meta>`, `<link>`, `<base>`
- **URL sanitization**: `href` and `src` values with disallowed schemes → `""`
- **CSS expression attacks**: `style` attrs with `expression()` or dangerous
  `url()` content removed
- **HTML comments**: stripped when `drop_comments: true`

## Policy Presets

| Preset               | Drop elements  | URL schemes              | Comments | Style |
|----------------------|----------------|--------------------------|----------|-------|
| `html_strict/0`      | 14 dangerous   | http, https, mailto      | Dropped  | Sanitized |
| `html_relaxed/0`     | 5 (script+iframes+plugins) | http, https, mailto, ftp | Kept | Sanitized |
| `html_passthrough/0` | None           | All (nil)                | Kept     | Not sanitized |

## XSS Protection

- Control characters stripped from URLs before scheme check
- Case-insensitive element matching (`<SCRIPT>`, `<Script>`)
- All `on*` attribute variants stripped

## Architecture

Uses regex-based pattern matching for portability. No native DOM parser required.
For higher fidelity in environments with a real HTML parser, use the DOM adapter
approach (not yet implemented in this Elixir port).

## Dependencies

None. Elixir stdlib only.

## Spec

Implements [TE02 — Document Sanitization](../../../specs/TE02-document-sanitization.md), Stage 2.
