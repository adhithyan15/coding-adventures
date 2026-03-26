# document-ast-to-html (Go)

An HTML renderer for the Document AST (spec TE00/TE02). Converts a
`DocumentNode` into a CommonMark-compliant HTML string.

## Usage

```go
import (
    documentast "github.com/adhithyan15/coding-adventures/code/packages/go/document-ast"
    renderer "github.com/adhithyan15/coding-adventures/code/packages/go/document-ast-to-html"
)

doc := &documentast.DocumentNode{ /* ... */ }

// Render with raw HTML passthrough (spec-compliant):
html := renderer.ToHtml(doc, renderer.RenderOptions{})

// Render with raw HTML stripped (safe for untrusted input):
safeHtml := renderer.ToHtml(doc, renderer.RenderOptions{Sanitize: true})
```

For most use-cases prefer the higher-level `commonmark` package which wires
parsing and rendering together.

## Rendering rules

| Node type        | Output                                              |
|------------------|-----------------------------------------------------|
| `HeadingNode`    | `<h1>`…`<h6>`                                       |
| `ParagraphNode`  | `<p>…</p>` (suppressed in tight list context)       |
| `CodeBlockNode`  | `<pre><code [class="language-X"]>…</code></pre>`    |
| `BlockquoteNode` | `<blockquote>\n…</blockquote>`                      |
| `ListNode`       | `<ul>` or `<ol [start="N"]>`                        |
| `ListItemNode`   | `<li>…</li>`                                        |
| `ThematicBreakNode` | `<hr />`                                         |
| `RawBlockNode`   | verbatim if `format="html"`, skipped otherwise      |
| `TextNode`       | HTML-escaped text                                   |
| `EmphasisNode`   | `<em>…</em>`                                        |
| `StrongNode`     | `<strong>…</strong>`                                |
| `CodeSpanNode`   | `<code>…</code>`                                    |
| `LinkNode`       | `<a href="…" [title="…"]>…</a>`                     |
| `ImageNode`      | `<img src="…" alt="…" [title="…"] />`               |
| `AutolinkNode`   | `<a href="…">…</a>`                                 |
| `RawInlineNode`  | verbatim if `format="html"`, skipped otherwise      |
| `HardBreakNode`  | `<br />\n`                                          |
| `SoftBreakNode`  | `\n`                                                |

### Tight vs loose lists

A tight list suppresses `<p>` wrappers around list item content; a loose list
includes them. The tightness is determined by the parser and stored in
`ListNode.Tight`.

### URL sanitisation

When `RenderOptions.Sanitize` is true, `javascript:`, `vbscript:`, `data:`, and
`blob:` URL schemes are blocked for `href`/`src` attributes.

## Stack position

```
document-ast-to-html   ← you are here
         ↑
   document-ast  (shared AST types, consumed from commonmark-parser output)
```

## Dependencies

- `document-ast` — shared AST node types

## Running tests

```
go test ./... -v -cover
```
