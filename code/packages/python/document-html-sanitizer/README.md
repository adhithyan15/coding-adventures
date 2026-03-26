# coding-adventures-document-html-sanitizer

A regex-based HTML string sanitizer with no external dependencies.

## What This Package Does

The HTML sanitizer operates on an **opaque HTML string** — it doesn't know how
the HTML was produced. It's a string-in, string-out transformation that removes
dangerous elements, strips event handler attributes, and sanitizes URL schemes.

**No dependency on `document-ast`** — this package is independently deployable
in any Python context.

Pipeline position (optional second stage):

```
parse(markdown)         → DocumentNode  (TE01)
       ↓
sanitize(doc, STRICT)  → DocumentNode  (TE02, stage 1 — document-ast-sanitizer)
       ↓
to_html(doc)            → str           (TE00)
       ↓
sanitize_html(html, .) → str           (TE02, stage 2 — this package)
```

Or standalone for HTML from external sources:

```python
safe = sanitize_html(cms_api_response_body, HTML_STRICT)
```

## Installation

```bash
pip install coding-adventures-document-html-sanitizer
```

## Usage

```python
from coding_adventures_document_html_sanitizer import (
    sanitize_html,
    HTML_STRICT,
    HTML_RELAXED,
    HTML_PASSTHROUGH,
)

# Remove all dangerous content from untrusted HTML
safe = sanitize_html(raw_html, HTML_STRICT)

# Authenticated users — allow more
safe = sanitize_html(rich_text_html, HTML_RELAXED)

# Trusted HTML — no changes
unchanged = sanitize_html(rendered_doc_html, HTML_PASSTHROUGH)
```

## Named Presets

| Preset | Drop elements | URL schemes | Comments | Style |
|--------|--------------|-------------|----------|-------|
| `HTML_STRICT` | script, style, iframe, form, meta, base, link, ... | http, https, mailto | dropped | sanitized |
| `HTML_RELAXED` | script, iframe, object, embed, applet | http, https, mailto, ftp | kept | sanitized |
| `HTML_PASSTHROUGH` | (none) | any | kept | kept |

## What Gets Removed

### Elements (including all nested content)

By default (`HTML_STRICT`), these elements are dropped along with everything inside them:

| Element | Risk |
|---------|------|
| `<script>` | Direct JavaScript execution |
| `<style>` | CSS expression() attacks |
| `<iframe>` | Framing attacks, clickjacking |
| `<object>` / `<embed>` | Plugin execution |
| `<applet>` | Java applet execution |
| `<form>` | CSRF, credential phishing |
| `<meta>` | Redirect attacks |
| `<base>` | Base URL hijacking |
| `<link>` | CSS import, DNS prefetch |
| `<noscript>` | Parser context abuse |

### Attributes

All `on*` event handler attributes are always stripped:
- `onclick`, `onload`, `onerror`, `onfocus`, `onmouseover`, etc.

Named dangerous attributes:
- `srcdoc` — iframe srcdoc XSS
- `formaction` — overrides form action URL

### URL Schemes

In `href` and `src` attributes, URLs with disallowed schemes are replaced with `""`:
- `javascript:alert(1)` → `href=""`
- `data:text/html,...` → `href=""`
- `vbscript:MsgBox(1)` → `href=""`

Control character bypasses are neutralised before scheme extraction:
- `java\x00script:` → `javascript:` → blocked
- `\u200bjavascript:` → `javascript:` → blocked

### CSS Injection

`style` attributes containing dangerous CSS are stripped entirely:
- `width: expression(alert(1))` → (attribute removed)
- `background: url(javascript:alert(1))` → (attribute removed)

## Custom Policy

```python
from coding_adventures_document_html_sanitizer import HtmlSanitizationPolicy
import dataclasses

# Start from HTML_STRICT, allow ftp links
my_policy = dataclasses.replace(HTML_STRICT, allowed_url_schemes=("http", "https", "ftp"))
safe = sanitize_html(html, my_policy)
```

## Spec

[TE02 — Document Sanitization](../../specs/TE02-document-sanitization.md)
