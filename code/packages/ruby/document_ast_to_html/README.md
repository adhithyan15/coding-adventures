# coding_adventures_document_ast_to_html

HTML renderer for the Document AST — converts a `DocumentNode` tree into a
CommonMark-compliant HTML string.

## Overview

`coding_adventures_document_ast_to_html` accepts any `DocumentNode` tree
produced by a parser that conforms to the **TE00 — Document AST** spec and
renders it to HTML following the CommonMark specification Appendix C rendering
rules.

Because the renderer only depends on the format-agnostic `DocumentNode` IR, it
can accept input from any front-end parser (Markdown, AsciiDoc, HTML, ...) —
not just the CommonMark parser.

## Dependency Diagram

```
[DocumentNode AST]  ←  any parser that implements TE00
        ↓
  DocumentAstToHtml (this package)
        ↓
    HTML string
```

## Node → HTML Mapping

### Block Nodes

| Node | HTML |
|------|------|
| `DocumentNode` | rendered children (no wrapper element) |
| `HeadingNode` (level 1–6) | `<h1>…</h1>` … `<h6>…</h6>` |
| `ParagraphNode` | `<p>…</p>` (suppressed in tight list context) |
| `CodeBlockNode` | `<pre><code [class="language-X"]>…</code></pre>` |
| `BlockquoteNode` | `<blockquote>\n…</blockquote>` |
| `ListNode` | `<ul>` or `<ol [start="N"]>` |
| `ListItemNode` | `<li>…</li>` |
| `ThematicBreakNode` | `<hr />` |
| `RawBlockNode` | verbatim if `format: "html"`, skipped otherwise |

### Inline Nodes

| Node | HTML |
|------|------|
| `TextNode` | HTML-escaped text |
| `EmphasisNode` | `<em>…</em>` |
| `StrongNode` | `<strong>…</strong>` |
| `CodeSpanNode` | `<code>…</code>` |
| `LinkNode` | `<a href="…" [title="…"]>…</a>` |
| `ImageNode` | `<img src="…" alt="…" [title="…"] />` |
| `AutolinkNode` | `<a href="[mailto:]…">…</a>` |
| `RawInlineNode` | verbatim if `format: "html"`, skipped otherwise |
| `HardBreakNode` | `<br />\n` |
| `SoftBreakNode` | `\n` |

## Tight vs Loose Lists

CommonMark distinguishes tight lists (no blank lines between items) from loose
lists (blank-line-separated items or items containing multiple blocks).

In a **tight** list, the `<p>` wrapper around paragraph content is suppressed:

```
Tight:  <li>item text</li>
Loose:  <li><p>item text</p></li>
```

The `tight` flag on `ListNode` controls this behaviour.

## Installation

```ruby
gem "coding_adventures_document_ast_to_html"
```

## Usage

```ruby
require "coding_adventures_document_ast_to_html"
require "coding_adventures_document_ast"

include CodingAdventures::DocumentAst

doc = DocumentNode.new(children: [
  HeadingNode.new(level: 1, children: [TextNode.new(value: "Hello")]),
  ParagraphNode.new(children: [TextNode.new(value: "World")])
])

html = CodingAdventures::DocumentAstToHtml.to_html(doc)
# => "<h1>Hello</h1>\n<p>World</p>\n"
```

### Sanitized output (strip raw HTML)

```ruby
html = CodingAdventures::DocumentAstToHtml.to_html(doc, sanitize: true)
```

When `sanitize: true`, all `RawBlockNode` and `RawInlineNode` content is
stripped. Use this when rendering untrusted user-supplied Markdown.

## Security

- Text content and attribute values are HTML-escaped via `escape_html`.
- `RawBlockNode`/`RawInlineNode` content passes through verbatim by default
  (required for spec compliance) — pass `sanitize: true` for user content.
- Link and image URLs are sanitized to block dangerous schemes:
  `javascript:`, `vbscript:`, `data:`, `blob:`.
- Autolink URLs are percent-encoded to prevent injection of unencoded characters.

## Spec

Implements spec **TE02 — Document AST to HTML** and depends on **TE00 — Document AST**.

## Requirements

- Ruby >= 3.2.0
- `coding_adventures_document_ast` ~> 0.1
