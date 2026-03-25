# document-ast-to-html

Render a Document AST to an HTML string — CommonMark-compliant HTML back-end.

## What it does

Walks a `DocumentNode` tree (from the `document-ast` crate) and produces an HTML fragment string. Every node type maps to CommonMark-specified HTML output.

```rust
use document_ast_to_html::{to_html, RenderOptions};
use commonmark_parser::parse;

let doc = parse("# Hello\n\nWorld *with* emphasis.\n");
let html = to_html(&doc, &Default::default());
// → "<h1>Hello</h1>\n<p>World <em>with</em> emphasis.</p>\n"
```

## Node mapping

| Node | HTML output |
|------|------------|
| `DocumentNode` | rendered children |
| `HeadingNode` (level N) | `<hN>…</hN>` |
| `ParagraphNode` | `<p>…</p>` (or bare text in tight lists) |
| `CodeBlockNode` | `<pre><code [class="language-X"]>…</code></pre>` |
| `BlockquoteNode` | `<blockquote>\n…</blockquote>` |
| `ListNode` (ordered) | `<ol [start="N"]>\n…</ol>` |
| `ListNode` (unordered) | `<ul>\n…</ul>` |
| `ListItemNode` | `<li>…</li>` |
| `ThematicBreakNode` | `<hr />` |
| `RawBlockNode` (html) | verbatim value |
| `TextNode` | HTML-escaped text |
| `EmphasisNode` | `<em>…</em>` |
| `StrongNode` | `<strong>…</strong>` |
| `CodeSpanNode` | `<code>…</code>` |
| `LinkNode` | `<a href="…" [title="…"]>…</a>` |
| `ImageNode` | `<img src="…" alt="…" [title="…"] />` |
| `AutolinkNode` | `<a href="[mailto:]…">…</a>` |
| `RawInlineNode` (html) | verbatim value |
| `HardBreakNode` | `<br />\n` |
| `SoftBreakNode` | `\n` |

## Tight vs loose lists

A tight list suppresses `<p>` tags in list items:

```html
<!-- tight -->
<ul>
<li>item 1</li>
<li>item 2</li>
</ul>

<!-- loose (items separated by blank lines) -->
<ul>
<li><p>item 1</p></li>
<li><p>item 2</p></li>
</ul>
```

## Security

Raw HTML passthrough (`RawBlockNode` and `RawInlineNode` with `format = "html"`) is enabled by default for CommonMark compliance. **When rendering untrusted Markdown**, use:

```rust
use document_ast_to_html::RenderOptions;

let opts = RenderOptions { sanitize: true };
let html = to_html(&doc, &opts);
// All raw HTML is stripped — XSS-safe
```

Link and image URLs are always sanitized — `javascript:`, `vbscript:`, `data:`, and `blob:` schemes are blocked regardless of the `sanitize` option.

## Utility functions

```rust
use document_ast_to_html::{escape_html, sanitize_url, normalize_url_for_attr};

// HTML-escape text for attribute values and content
assert_eq!(escape_html("<script>"), "&lt;script&gt;");

// Strip dangerous URL schemes
assert_eq!(sanitize_url("javascript:alert(1)"), "");

// Percent-encode URL characters for HTML attributes
assert_eq!(normalize_url_for_attr("foo bar"), "foo%20bar");
```

## Version

0.1.0
