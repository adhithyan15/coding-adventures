# coding_adventures.document_ast_sanitizer

Policy-driven Document AST sanitizer for Lua. Part of the TE02 sanitization
pipeline specification in the coding-adventures monorepo.

## What it does

Takes a `DocumentNode` tree (as produced by `commonmark_parser`) and returns a
new, sanitized tree with all policy violations removed or neutralised. The
input is never mutated.

```
parse(markdown)          ← TE01 — CommonMark Parser
      ↓
sanitize(doc, policy)    ← TE02 — document_ast_sanitizer (this package)
      ↓
to_html(doc)             ← TE00 — document_ast_to_html
      ↓
sanitize_html(html, pol) ← TE02 — document_html_sanitizer (optional second stage)
```

## Why a separate package?

The renderer's job is faithful conversion of AST to HTML — it should not
second-guess policy decisions. The sanitizer's job is policy enforcement on
structured data. Separating these concerns means:

- The sanitizer is independently testable without a renderer.
- Custom policies (allow HTML blocks, clamp headings to h2+, etc.) are
  composable plain data objects rather than boolean flags.
- Non-HTML renderers (PDF, plain text) can sanitize the AST before rendering.

## Installation

```bash
luarocks install coding-adventures-document-ast-sanitizer
```

Or from source:

```bash
luarocks make coding-adventures-document-ast-sanitizer-0.1.0-1.rockspec
```

## Usage

```lua
local ast      = require("coding_adventures.document_ast")
local sanitizer = require("coding_adventures.document_ast_sanitizer")
local cm        = require("coding_adventures.commonmark_parser")
local html      = require("coding_adventures.document_ast_to_html")

-- User-generated content — strict policy
local safe = sanitizer.sanitize(cm.parse(user_markdown), sanitizer.STRICT)
local output = html.to_html(safe)

-- Documentation — pass through everything
local doc = sanitizer.sanitize(cm.parse(trusted_markdown), sanitizer.PASSTHROUGH)

-- Custom policy
local my_policy = sanitizer.with_defaults({
  allowRawBlockFormats = { "html" },
  minHeadingLevel      = 2,
  allowedUrlSchemes    = { "http", "https" },
})
local custom_doc = sanitizer.sanitize(cm.parse(markdown), my_policy)
```

## Named Presets

### `STRICT` — user-generated content

```lua
sanitizer.STRICT = {
  allowRawBlockFormats    = "drop-all",
  allowRawInlineFormats   = "drop-all",
  allowedUrlSchemes       = { "http", "https", "mailto" },
  transformImageToText    = true,    -- images → alt text
  minHeadingLevel         = 2,       -- reserve h1 for page title
  maxHeadingLevel         = 6,
  ...
}
```

### `RELAXED` — authenticated users, internal wikis

```lua
sanitizer.RELAXED = {
  allowRawBlockFormats  = { "html" },   -- HTML raw blocks OK
  allowRawInlineFormats = { "html" },
  allowedUrlSchemes     = { "http", "https", "mailto", "ftp" },
  ...
}
```

### `PASSTHROUGH` — fully trusted content

```lua
sanitizer.PASSTHROUGH = {
  allowRawBlockFormats = "passthrough",
  allowedUrlSchemes    = false,   -- any scheme allowed
  ...
}
```

## Policy Reference

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `allowRawBlockFormats` | `"drop-all"` \| `"passthrough"` \| `{string}` | `"passthrough"` | Which raw block formats to keep |
| `allowRawInlineFormats` | same | `"passthrough"` | Which raw inline formats to keep |
| `allowedUrlSchemes` | `{string}` \| `false` | `false` | Scheme allowlist for links/images/autolinks |
| `dropLinks` | boolean | `false` | Promote link children to parent (remove `<a>`) |
| `dropImages` | boolean | `false` | Drop image nodes entirely |
| `transformImageToText` | boolean | `false` | Replace image with alt text node |
| `maxHeadingLevel` | 1–6 \| `"drop"` | `6` | Clamp or drop headings |
| `minHeadingLevel` | 1–6 | `1` | Promote headings shallower than this |
| `dropBlockquotes` | boolean | `false` | Drop blockquote nodes |
| `dropCodeBlocks` | boolean | `false` | Drop code block nodes |
| `transformCodeSpanToText` | boolean | `false` | Convert code spans to text |

## URL Scheme Handling

Before checking the scheme, the sanitizer strips C0 control characters
(bytes 0x00–0x1F), DEL (0x7F), and Unicode invisible code points (U+200B,
U+200C, U+200D, U+2060, U+FEFF). This blocks bypass vectors like
`java\x00script:alert(1)`.

Relative URLs (no scheme component) always pass through regardless of the
`allowedUrlSchemes` list.

## Running tests

```bash
luarocks make coding-adventures-document-ast-sanitizer-0.1.0-1.rockspec
busted spec/ --verbose
```

## Spec

TE02 — Document Sanitization (`code/specs/TE02-document-sanitization.md`)
