# commonmark (Go)

A thin pipeline package that combines the CommonMark parser and HTML renderer
into a single, convenient API. 100% conformant with the CommonMark 0.31.2
specification (all 652 spec examples pass).

## Quick start

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/commonmark"

// Parse Markdown to HTML:
html := commonmark.ToHtml("# Hello\n\nWorld *with* emphasis.\n")
// → "<h1>Hello</h1>\n<p>World <em>with</em> emphasis.</p>\n"

// Parse Markdown to the Document AST:
doc := commonmark.Parse("# Hello\n")
// doc.Children[0].NodeType() == "heading"

// Render user-provided Markdown safely (raw HTML is stripped):
safeHtml := commonmark.ToHtmlSafe("Hello\n\n<script>evil</script>\n")
// → "<p>Hello</p>\n"
```

## API

### `Parse(markdown string) *documentast.DocumentNode`

Parses a CommonMark Markdown string into a `DocumentNode` AST (TE00). All link
references are resolved and all inline markup is fully parsed. Use this when you
need to post-process or inspect the AST before rendering.

### `ToHtml(markdown string) string`

Parses Markdown and renders it to an HTML string. Raw HTML passthrough is
enabled (required for full CommonMark spec compliance). **Do not use this for
untrusted user content.**

### `ToHtmlSafe(markdown string) string`

Like `ToHtml` but strips all raw HTML from the output. Use this when rendering
untrusted (user-provided) Markdown.

### `VERSION` / `COMMONMARK_VERSION`

Package version (`"0.1.0"`) and the CommonMark spec version supported
(`"0.31.2"`).

## Architecture

```
       commonmark          ← you are here (thin pipeline)
      /           \
commonmark-parser  document-ast-to-html
      \           /
      document-ast
```

The actual work is done by:
- **`commonmark-parser`** — two-phase Markdown → DocumentNode parser.
- **`document-ast-to-html`** — DocumentNode → HTML string renderer.
- **`document-ast`** — shared intermediate representation types (TE00).

## Spec conformance

All 652 CommonMark 0.31.2 spec examples pass (`TestCommonMarkSpec`). The spec
JSON is shipped alongside this package as `spec.json`.

## Running tests

```
go test ./... -v -cover
```
