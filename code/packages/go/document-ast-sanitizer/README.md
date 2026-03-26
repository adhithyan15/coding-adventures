# document-ast-sanitizer (Go)

A pure, immutable, policy-driven sanitizer for the Document AST. Sits between
the parser and the renderer in the document pipeline.

```
parse(markdown)          ← commonmark-parser
      ↓
sanitize(doc, policy)    ← this package
      ↓
toHtml(doc)              ← document-ast-to-html
      ↓
sanitizeHtml(html, pol)  ← document-html-sanitizer (optional second pass)
```

Spec: [TE02 — Document Sanitization](../../../../specs/TE02-document-sanitization.md)

## Why a separate package?

The parser's job is to faithfully decode Markdown — it should not silently
discard content the author intended. The renderer's job is to faithfully encode
the AST as HTML — it should not make policy decisions. Sanitization belongs to
the caller who knows the security context.

This package enforces that separation by providing a standalone, independently
testable unit.

## Installation

```bash
go get github.com/adhithyan15/coding-adventures/code/packages/go/document-ast-sanitizer
```

Replace with a local path in development:

```go
// go.mod
replace github.com/adhithyan15/coding-adventures/code/packages/go/document-ast-sanitizer => ../document-ast-sanitizer
```

## Quick Start

```go
import (
    sanitizer "github.com/adhithyan15/coding-adventures/code/packages/go/document-ast-sanitizer"
    parser    "github.com/adhithyan15/coding-adventures/code/packages/go/commonmark-parser"
    renderer  "github.com/adhithyan15/coding-adventures/code/packages/go/document-ast-to-html"
)

// User-generated content (comments, forum posts):
safe := sanitizer.Sanitize(parser.Parse(userMarkdown), sanitizer.STRICT)
html := renderer.ToHtml(safe, renderer.RenderOptions{})

// Documentation (fully trusted):
doc := sanitizer.Sanitize(parser.Parse(trustedMarkdown), sanitizer.PASSTHROUGH)
html := renderer.ToHtml(doc, renderer.RenderOptions{})
```

## Presets

### `STRICT` — user-generated content

Recommended for comments, forum posts, and chat messages from untrusted users.

- Drops all raw HTML/format passthrough (no `<script>` injection)
- Allows only `http`, `https`, and `mailto` URL schemes
- Converts images to alt text instead of rendering `<img>` tags
- Clamps headings to `h2`–`h6` (reserves `h1` for the page title)
- Keeps links, blockquotes, and code blocks

### `RELAXED` — semi-trusted content

Recommended for authenticated users and internal wikis.

- Allows HTML raw blocks (but drops LaTeX and other formats)
- Allows `http`, `https`, `mailto`, and `ftp` URL schemes
- Images pass through unchanged
- Headings unrestricted

### `PASSTHROUGH` — trusted content

No sanitization. Equivalent to not calling `Sanitize()` at all. Use for
documentation and static sites.

## Custom Policies

Policies are plain structs — compose them with struct literals:

```go
// Reserve h1 for the page title; allow ftp for file downloads
policy := sanitizer.RELAXED
policy.MinHeadingLevel = 2
policy.AllowedUrlSchemes = []string{"http", "https", "mailto", "ftp"}

safe := sanitizer.Sanitize(doc, policy)
```

## SanitizationPolicy Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `AllowRawBlockFormats` | `RawFormatPolicy` | `RawPassthrough` | Which raw block formats to allow |
| `AllowRawInlineFormats` | `RawFormatPolicy` | `RawPassthrough` | Which raw inline formats to allow |
| `AllowedUrlSchemes` | `[]string` | `nil` (all) | Allowed URL schemes in links/images |
| `AllowAllSchemes` | `bool` | `false` | Bypass scheme check entirely |
| `DropLinks` | `bool` | `false` | Remove links, promote children |
| `DropImages` | `bool` | `false` | Remove images entirely |
| `TransformImageToText` | `bool` | `false` | Replace images with alt text |
| `MaxHeadingLevel` | `int` | `0` (6) | Max heading level; -1 drops all |
| `MinHeadingLevel` | `int` | `0` (1) | Min heading level |
| `DropBlockquotes` | `bool` | `false` | Remove blockquotes |
| `DropCodeBlocks` | `bool` | `false` | Remove code blocks |
| `TransformCodeSpanToText` | `bool` | `false` | Convert code spans to text |

### `RawFormatPolicy`

```go
type RawFormatPolicy struct {
    Mode           RawFormatMode  // RawDropAll, RawPassthrough, or RawAllowList
    AllowedFormats []string       // Used when Mode == RawAllowList
}
```

## URL Scheme Sanitization

The sanitizer performs a two-step URL check to defeat bypass attacks:

1. **Strip invisible characters** — C0 controls (U+0000–U+001F) and zero-width
   characters (U+200B, U+200C, U+200D, U+2060, U+FEFF) are stripped before
   scheme extraction. This blocks attacks like `java\x00script:alert(1)`.

2. **Case-insensitive scheme check** — The scheme is lowercased before
   comparison, blocking `JAVASCRIPT:alert(1)`.

Relative URLs (no scheme) always pass through regardless of policy.

## Immutability Guarantee

`Sanitize()` never mutates the input `DocumentNode`. It always returns a
freshly constructed tree. You can safely pass the same document through
multiple sanitizers with different policies.

## Node Type Coverage

Every node type in the Document AST is handled explicitly. Unknown node types
are dropped (fail-safe). When new node types are added to the AST, this package
must be updated.

| Node Type | Behaviour |
|-----------|-----------|
| `DocumentNode` | Always recurse into children |
| `HeadingNode` | Clamp/promote level or drop |
| `ParagraphNode` | Recurse; drop if empty |
| `CodeBlockNode` | Drop if `DropCodeBlocks`, else keep |
| `BlockquoteNode` | Drop if `DropBlockquotes`, else recurse |
| `ListNode` | Recurse |
| `ListItemNode` | Recurse |
| `ThematicBreakNode` | Always keep |
| `RawBlockNode` | Apply `AllowRawBlockFormats` policy |
| `TextNode` | Always keep |
| `EmphasisNode` | Recurse; drop if empty |
| `StrongNode` | Recurse; drop if empty |
| `CodeSpanNode` | Keep or convert to text |
| `LinkNode` | Promote children, sanitize URL, or keep |
| `ImageNode` | Drop, convert to text, or sanitize URL |
| `AutolinkNode` | Drop or keep based on scheme |
| `RawInlineNode` | Apply `AllowRawInlineFormats` policy |
| `HardBreakNode` | Always keep |
| `SoftBreakNode` | Always keep |

## Test Coverage

```
go test ./... -coverprofile=coverage.out
go tool cover -func=coverage.out
```

Coverage target: 90%+. Current: 94.7%.

## Dependencies

- `github.com/adhithyan15/coding-adventures/code/packages/go/document-ast` — AST types only
- No other runtime dependencies
