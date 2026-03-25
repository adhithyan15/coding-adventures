# coding_adventures_document_html_sanitizer

Regex-based HTML string sanitizer — strips dangerous elements, attributes, and
URLs from an opaque HTML string with **no dependency on document-ast** (TE02, stage 2).

## What it does

Accepts any HTML string and returns a sanitized version. No DOM parsing required.
Uses regex-based pattern matching for portability across all target languages.

```text
sanitize(doc, policy)    ← TE02 stage 1 — document-ast-sanitizer
      ↓
to_html(doc)             ← TE00 — document-ast-to-html
      ↓
sanitize_html(html, pol) ← TE02 stage 2 — document-html-sanitizer (this crate)
      ↓
final output
```

## Key features

- **No document-ast dependency** — string in, string out, fully self-contained
- **Element removal with content** — `<script>alert(1)</script>` → removed entirely
- **Always-on event handler stripping** — `on*` attributes removed from all elements
- **URL scheme sanitization** — `javascript:` in href/src neutralized to `""`
- **CSS injection prevention** — `expression()` and `url(javascript:...)` in style attrs stripped
- **Comment stripping** — removes `<!-- … -->` including IE conditional comments

## Quick start

```rust
use coding_adventures_document_html_sanitizer::{sanitize_html, html_strict};

// Remove script elements and event handlers
let safe = sanitize_html(
    "<p>Hello</p><script>alert(1)</script>",
    &html_strict(),
);
assert_eq!(safe, "<p>Hello</p>");

// Neutralize javascript: URLs
let safe = sanitize_html(
    r#"<a href="javascript:alert(1)">click</a>"#,
    &html_strict(),
);
assert_eq!(safe, r#"<a href="">click</a>"#);
```

## Named presets

| Preset              | Use case                                      |
|---------------------|-----------------------------------------------|
| `html_strict()`     | Untrusted HTML from external sources          |
| `html_relaxed()`    | Authenticated users / internal tools          |
| `html_passthrough()`| No element/attribute restrictions (trusted)  |

Note: `html_passthrough()` still strips `on*` event handlers — this is a
core safety invariant, not a policy option.

## What `html_strict()` removes

| Element/Pattern        | Risk                                           |
|------------------------|------------------------------------------------|
| `<script>`             | Direct JavaScript execution                    |
| `<style>`              | CSS expression() attacks, data exfiltration   |
| `<iframe>`             | Framing attacks, clickjacking                  |
| `<object>`, `<embed>`  | Plugin execution                               |
| `<form>`, `<input>`    | CSRF, credential phishing                      |
| `<meta>`               | Redirect via http-equiv="refresh"              |
| `<base>`               | Base URL hijacking                             |
| `on*` attributes       | All event handlers (onclick, onload, etc.)     |
| `srcdoc`, `formaction` | Inline HTML injection, form action override   |
| `javascript:` in href  | URL-based JavaScript execution                 |
| `expression(` in style | CSS expression injection                       |
| HTML comments          | IE conditional comment attacks                 |

## Dependencies

- `regex = "1"` — for pattern matching

## Development

```bash
cargo build --manifest-path Cargo.toml
cargo test --manifest-path Cargo.toml -- --nocapture
cargo clippy
```
