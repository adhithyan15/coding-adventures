# @coding-adventures/document-ast

**Format-agnostic Intermediate Representation (IR) for structured documents.**

The Document AST is the LLVM IR of documents. It sits between front-end
parsers and back-end renderers, enabling any front-end to produce output for
any back-end without direct coupling.

```
Markdown ─────────────────────────────────► HTML
reStructuredText ────► Document AST ──────► PDF
HTML ─────────────────────────────────────► Plain text
DOCX ─────────────────────────────────────► DOCX
LaTeX ─────────────────────────────────────► LaTeX
```

Without a shared IR, every (source, target) pair needs its own converter:
**N × M** implementations. With a shared IR: **N + M** implementations.

Spec: [TE00 — Document AST](../../../../specs/TE00-document-ast.md)

---

## Installation

```bash
npm install @coding-adventures/document-ast
```

---

## Usage

This is a **types-only** package — there is no runtime code and no
dependencies. Import the types to annotate your parser output or renderer
input.

```typescript
import type { DocumentNode, BlockNode, InlineNode } from "@coding-adventures/document-ast";

// Count headings in a document
function countHeadings(doc: DocumentNode): number {
  return doc.children.filter(n => n.type === "heading").length;
}

// Exhaustive switch over all block types
function renderBlock(node: BlockNode): string {
  switch (node.type) {
    case "document":    return node.children.map(renderBlock).join("");
    case "heading":     return `<h${node.level}>...</h${node.level}>\n`;
    case "paragraph":   return `<p>...</p>\n`;
    case "code_block":  return `<pre><code>...</code></pre>\n`;
    case "blockquote":  return `<blockquote>...</blockquote>\n`;
    case "list":        return `<ul>...</ul>\n`;
    case "list_item":   return `<li>...</li>\n`;
    case "thematic_break": return `<hr />\n`;
    case "raw_block":
      // Emit verbatim only if format matches our output format
      return node.format === "html" ? node.value : "";
  }
}
```

---

## Node Types

### Block nodes (structural skeleton)

| Type | Description |
|------|-------------|
| `DocumentNode` | Root of every document |
| `HeadingNode` | Section heading, levels 1–6 |
| `ParagraphNode` | Block of prose |
| `CodeBlockNode` | Pre-formatted code block |
| `BlockquoteNode` | Quoted content |
| `ListNode` | Ordered or unordered list |
| `ListItemNode` | Single item within a list |
| `ThematicBreakNode` | Visual section separator (`<hr />`) |
| `RawBlockNode` | Verbatim pass-through for a specific back-end |

### Inline nodes (content within prose)

| Type | Description |
|------|-------------|
| `TextNode` | Plain text (HTML entities decoded) |
| `EmphasisNode` | Stressed emphasis (`<em>`) |
| `StrongNode` | Strong importance (`<strong>`) |
| `CodeSpanNode` | Inline code span |
| `LinkNode` | Hyperlink with resolved destination |
| `ImageNode` | Embedded image |
| `AutolinkNode` | URL or email as a direct link |
| `RawInlineNode` | Verbatim pass-through for a specific back-end |
| `HardBreakNode` | Forced line break (`<br />`) |
| `SoftBreakNode` | Soft line break (preserves source newlines) |

---

## Differences from the CommonMark AST (TE01)

| CommonMark AST | Document AST | Reason |
|---|---|---|
| `LinkDefinitionNode` | **Removed** | Markdown parse artifact; links are always resolved |
| `HtmlBlockNode { type: "html_block" }` | `RawBlockNode { type: "raw_block"; format: "html" }` | Generalise to any back-end format |
| `HtmlInlineNode { type: "html_inline" }` | `RawInlineNode { type: "raw_inline"; format: "html" }` | Generalise to any back-end format |
| All other node types | **Identical** | Semantically universal |

---

## Package Ecosystem

This package is the foundation of a three-package pipeline:

```
@coding-adventures/document-ast          ← you are here (types only)
        ↓ types                                   ↓ types
@coding-adventures/commonmark-parser     @coding-adventures/document-ast-to-html
  parse(markdown) → DocumentNode           toHtml(doc) → string
        ↓ depends on both
@coding-adventures/commonmark
  const html = toHtml(parse(markdown))
```

---

## Related Packages

- [`@coding-adventures/commonmark-parser`](../commonmark-parser/) — Markdown → Document AST
- [`@coding-adventures/document-ast-to-html`](../document-ast-to-html/) — Document AST → HTML
- [`@coding-adventures/commonmark`](../commonmark/) — convenience pipeline (parse + render)
