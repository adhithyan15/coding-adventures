# coding-adventures-document-ast-sanitizer

A policy-driven Document AST sanitizer for the `coding-adventures` document pipeline.

## What This Package Does

This package sits between the CommonMark parser and the HTML renderer in the document pipeline:

```
parse(markdown)          → DocumentNode  (TE01 — CommonMark Parser)
       ↓
sanitize(doc, policy)   → DocumentNode  (TE02 — this package)
       ↓
to_html(doc)             → str           (TE00 — document-ast-to-html)
```

It performs a **pure, immutable tree transformation** — every call to `sanitize()` returns a freshly constructed `DocumentNode`. The input is never mutated.

## Why a Separate Sanitization Package?

The `sanitize: boolean` option in `document-ast-to-html` was a design mistake:

1. **Boolean is too coarse** — you can't express "allow HTML raw blocks but not LaTeX blocks", "strip images but keep links", or "clamp headings to level 2".
2. **Wrong layer** — the renderer doesn't know whether the document is for a trusted editor or an untrusted comment thread.
3. **Not composable** — non-HTML renderers (PDF, plain text) can't sanitize before rendering.

This package extracts sanitization into a dedicated, testable, language-portable layer.

## Installation

```bash
pip install coding-adventures-document-ast-sanitizer
```

## Usage

```python
from coding_adventures_document_ast_sanitizer import sanitize, STRICT, RELAXED, PASSTHROUGH

# User-generated content (forum posts, comments, chat messages)
safe = sanitize(parse(user_markdown), STRICT)
html = to_html(safe)

# Authenticated user content (internal wikis, dashboards)
safe = sanitize(parse(editor_markdown), RELAXED)

# Fully trusted content (documentation, static sites)
doc = sanitize(parse(trusted_markdown), PASSTHROUGH)
```

### Custom Policy

```python
from coding_adventures_document_ast_sanitizer import SanitizationPolicy, RELAXED
import dataclasses

# Start from RELAXED, but reserve h1 for the page title
my_policy = dataclasses.replace(RELAXED, min_heading_level=2)
safe = sanitize(parse(markdown), my_policy)
```

## Named Presets

| Preset | Use Case | Raw blocks | URL schemes | Images | Headings |
|--------|----------|------------|-------------|--------|----------|
| `STRICT` | User-generated content | drop-all | http, https, mailto | → alt text | h2–h6 |
| `RELAXED` | Authenticated users | html only | http, https, mailto, ftp | kept | unrestricted |
| `PASSTHROUGH` | Trusted content | all | any | kept | unrestricted |

## Policy Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `allow_raw_block_formats` | `"drop-all" \| "passthrough" \| tuple[str,...]` | `"passthrough"` | Which `RawBlockNode` formats survive |
| `allow_raw_inline_formats` | `"drop-all" \| "passthrough" \| tuple[str,...]` | `"passthrough"` | Which `RawInlineNode` formats survive |
| `allowed_url_schemes` | `tuple[str,...] \| None` | `("http","https","mailto","ftp")` | Permitted URL schemes; `None` = any |
| `drop_links` | `bool` | `False` | Drop links, promote text children to parent |
| `drop_images` | `bool` | `False` | Drop images entirely |
| `transform_image_to_text` | `bool` | `False` | Replace image with alt text |
| `max_heading_level` | `int \| "drop"` | `6` | Clamp deep headings or drop all |
| `min_heading_level` | `int` | `1` | Promote shallow headings |
| `drop_blockquotes` | `bool` | `False` | Drop blockquotes entirely |
| `drop_code_blocks` | `bool` | `False` | Drop code blocks |
| `transform_code_span_to_text` | `bool` | `False` | Convert code spans to plain text |

## URL Scheme Sanitization

The sanitizer strips C0 control characters (U+0000–U+001F) and zero-width characters (U+200B, U+200C, U+200D, U+2060, U+FEFF) from URLs **before** scheme extraction. This defeats bypass attacks like:

- `java\x00script:alert(1)` → stripped to `javascript:` → blocked
- `\u200bjavascript:alert(1)` → stripped → blocked
- `JAVASCRIPT:alert(1)` → lowercased → blocked

## Empty Container Cleanup

When all children of a container node are dropped, the container itself is dropped:

```python
# Paragraph containing only a dropped raw inline → paragraph dropped too
para(raw_inline("html", "<em>"))  →  (removed)

# Exception: DocumentNode is never dropped — empty documents are valid
sanitize(doc(raw_block("html", "<div>")), STRICT)
# → DocumentNode { children: [] }
```

## Spec

[TE02 — Document Sanitization](../../specs/TE02-document-sanitization.md)
