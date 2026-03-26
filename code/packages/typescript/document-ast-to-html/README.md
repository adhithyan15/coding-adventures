# @coding-adventures/document-ast-to-html

**Document AST → HTML renderer.**

Converts a [`DocumentNode`](../document-ast/) from the format-agnostic Document
AST IR into a valid HTML string. Works with any front-end that produces the
Document AST — not just Markdown.

Spec: [TE00 — Document AST](../../../../specs/TE00-document-ast.md)

---

## Installation

```bash
npm install @coding-adventures/document-ast-to-html
```

---

## Usage

```typescript
import { parse }  from "@coding-adventures/commonmark-parser";
import { toHtml } from "@coding-adventures/document-ast-to-html";

const html = toHtml(parse("# Hello\n\nWorld *with* **emphasis**.\n"));
// → "<h1>Hello</h1>\n<p>World <em>with</em> <strong>emphasis</strong>.</p>\n"
```

### Consuming a hand-built AST

```typescript
import type { DocumentNode } from "@coding-adventures/document-ast";
import { toHtml } from "@coding-adventures/document-ast-to-html";

const doc: DocumentNode = {
  type: "document",
  children: [
    {
      type: "heading",
      level: 1,
      children: [{ type: "text", value: "Hello" }],
    },
  ],
};

toHtml(doc);
// → "<h1>Hello</h1>\n"
```

---

## Node → HTML mapping

| Node type | HTML output |
|-----------|-------------|
| `DocumentNode` | rendered children |
| `HeadingNode` (level N) | `<hN>…</hN>` |
| `ParagraphNode` | `<p>…</p>` (or bare content in tight list) |
| `CodeBlockNode` | `<pre><code [class="language-X"]>…</code></pre>` |
| `BlockquoteNode` | `<blockquote>\n…</blockquote>` |
| `ListNode` (unordered) | `<ul>\n…</ul>` |
| `ListNode` (ordered) | `<ol [start="N"]>\n…</ol>` |
| `ListItemNode` | `<li>…</li>` |
| `ThematicBreakNode` | `<hr />` |
| `RawBlockNode { format: "html" }` | value verbatim |
| `RawBlockNode { format: other }` | *(skipped)* |
| `TextNode` | HTML-escaped text |
| `EmphasisNode` | `<em>…</em>` |
| `StrongNode` | `<strong>…</strong>` |
| `CodeSpanNode` | `<code>…</code>` |
| `LinkNode` | `<a href="…" [title="…"]>…</a>` |
| `ImageNode` | `<img src="…" alt="…" [title="…"] />` |
| `AutolinkNode` | `<a href="[mailto:]…">…</a>` |
| `RawInlineNode { format: "html" }` | value verbatim |
| `RawInlineNode { format: other }` | *(skipped)* |
| `HardBreakNode` | `<br />\n` |
| `SoftBreakNode` | `\n` |

---

## Security

- All text content and attribute values are HTML-escaped (`& < > " '`).
- `RawBlockNode` and `RawInlineNode` with `format: "html"` pass through verbatim
  (this is intentional and spec-required).
- Link and image URLs are sanitized: `javascript:`, `vbscript:`, and `data:`
  schemes are replaced with an empty string.

---

## Related Packages

- [`@coding-adventures/document-ast`](../document-ast/) — the Document AST type definitions
- [`@coding-adventures/commonmark-parser`](../commonmark-parser/) — Markdown → Document AST
- [`@coding-adventures/commonmark`](../commonmark/) — convenience pipeline (parse + render)
