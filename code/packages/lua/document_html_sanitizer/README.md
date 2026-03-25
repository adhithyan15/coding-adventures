# coding_adventures.document_html_sanitizer

Pattern-based HTML string sanitizer for Lua. Part of the TE02 sanitization
pipeline specification in the coding-adventures monorepo.

**No dependency on `document_ast`** — string in, string out.

## What it does

Takes an opaque HTML string from any source and returns a cleaned string with
dangerous elements, attributes, and URLs removed. Useful as a safety net after
HTML rendering, or when HTML arrives from external systems (CMS APIs, user
pastes, etc.).

```
parse(markdown)          ← TE01 — CommonMark Parser
      ↓
sanitize(doc, policy)    ← TE02 — document_ast_sanitizer (preferred stage)
      ↓
to_html(doc)             ← TE00 — document_ast_to_html
      ↓
sanitize_html(html, pol) ← TE02 — document_html_sanitizer (this package)
      ↓
final output
```

## Why pattern-based?

The spec (TE02 §Decision 5) mandates pattern-based string operations for
portability. Go, Python, Rust, Elixir, Lua, and edge JS runtimes have no
shared DOM API. Pattern-based sanitization gives the same logic everywhere
without external dependencies.

## Installation

```bash
luarocks install coding-adventures-document-html-sanitizer
```

Or from source:

```bash
luarocks make coding-adventures-document-html-sanitizer-0.1.0-1.rockspec
```

## Usage

```lua
local html_san = require("coding_adventures.document_html_sanitizer")

-- Sanitize HTML from an external CMS API
local safe = html_san.sanitize_html(cms_response.html, html_san.HTML_STRICT)

-- Two-stage belt-and-suspenders pipeline
local cm        = require("coding_adventures.commonmark_parser")
local ast_san   = require("coding_adventures.document_ast_sanitizer")
local renderer  = require("coding_adventures.document_ast_to_html")

local ast      = cm.parse(user_markdown)
local safe_ast = ast_san.sanitize(ast, ast_san.STRICT)
local html     = renderer.to_html(safe_ast)
local final    = html_san.sanitize_html(html, html_san.HTML_STRICT)
```

## Named Presets

### `HTML_STRICT` — untrusted external HTML

Drops: `script`, `style`, `iframe`, `object`, `embed`, `applet`, `form`,
`input`, `button`, `select`, `textarea`, `noscript`, `meta`, `link`, `base`.

Strips: all `on*` event handlers, `srcdoc`, `formaction`.

URL schemes: `http`, `https`, `mailto` only.

Comments: stripped.

### `HTML_RELAXED` — authenticated users / internal tools

Drops: `script`, `iframe`, `object`, `embed`, `applet`.

Strips: `on*`, `srcdoc`, `formaction`.

URL schemes: `http`, `https`, `mailto`, `ftp`.

Comments: kept.

### `HTML_PASSTHROUGH` — no sanitization

Nothing stripped. Useful for trusted content or debugging.

## What gets stripped

### Dangerous elements (content removed entirely)

| Element | Risk |
|---------|------|
| `<script>` | Direct JavaScript execution |
| `<style>` | CSS expression() attacks |
| `<iframe>` | Framing / clickjacking |
| `<object>`, `<embed>`, `<applet>` | Plugin execution |
| `<form>`, `<input>` | CSRF, credential phishing |
| `<meta>` | Redirect via `http-equiv="refresh"` |
| `<base>` | Base URL hijacking |
| `<link>` | CSS import, DNS prefetch |

### Dangerous attributes (stripped from all elements)

| Pattern | Risk |
|---------|------|
| `on*` | All event handlers (`onclick`, `onload`, etc.) |
| `srcdoc` | Inline HTML frame content |
| `formaction` | Override form submission URL |

### CSS injection

When `sanitize_style_attributes = true`, any `style` attribute containing
`expression(` or `url(` with a non-http/https argument is stripped entirely.

### URL schemes

`href` and `src` attributes with disallowed schemes are replaced with `""`.
Relative URLs always pass through.

## URL Bypass Vectors Defended

Before scheme comparison, the sanitizer strips:
- C0 control chars (bytes 0x00–0x1F) and DEL (0x7F)
- Unicode zero-width chars: U+200B, U+200C, U+200D, U+2060, U+FEFF

This blocks `java\x00script:alert(1)` and similar bypasses.

## Running tests

```bash
luarocks make coding-adventures-document-html-sanitizer-0.1.0-1.rockspec
busted spec/ --verbose
```

## Limitations

This is a pattern-based sanitizer, not a full HTML parser. It handles the
common XSS vectors from the TE02 spec. For adversarial or deeply malformed
HTML, the AST sanitizer pipeline is more reliable because it operates on
structured data before rendering.

## Spec

TE02 — Document Sanitization (`code/specs/TE02-document-sanitization.md`)
