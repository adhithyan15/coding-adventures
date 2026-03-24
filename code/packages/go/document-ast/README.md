# document-ast (Go)

Format-agnostic Intermediate Representation (IR) for structured documents.

## Overview

The Document AST is the "LLVM IR of documents." It sits between front-end
parsers (Markdown, RST, HTML, DOCX) and back-end renderers (HTML, PDF,
plain text, LaTeX). Every front-end produces this IR; every back-end
consumes it. With a shared IR, **N front-ends × M back-ends** requires only
**N + M** implementations instead of N × M.

```
Markdown ──────────────────────────► HTML
reStructuredText ──► Document AST ──► PDF
HTML ───────────────────────────────► Plain text
DOCX ───────────────────────────────► DOCX
```

This is a **types-only** package — there is no runtime logic and no
dependencies. Import it to annotate the AST values produced by a front-end
or consumed by a back-end.

## Installation

```bash
go get github.com/adhithyan15/coding-adventures/code/packages/go/document-ast
```

## Usage

```go
import documentast "github.com/adhithyan15/coding-adventures/code/packages/go/document-ast"

func countHeadings(doc *documentast.DocumentNode) int {
    count := 0
    for _, child := range doc.Children {
        if _, ok := child.(*documentast.HeadingNode); ok {
            count++
        }
    }
    return count
}
```

## Node Hierarchy

### Block Nodes

| Type | Description |
|------|-------------|
| `DocumentNode` | Root of every document |
| `HeadingNode` | Section heading (Level 1–6) |
| `ParagraphNode` | Block of prose |
| `CodeBlockNode` | Literal code / pre-formatted text |
| `BlockquoteNode` | Quoted content |
| `ListNode` | Ordered or unordered list |
| `ListItemNode` | Single item in a list |
| `ThematicBreakNode` | Horizontal rule (`---`) |
| `RawBlockNode` | Verbatim content for a specific back-end |

### Inline Nodes

| Type | Description |
|------|-------------|
| `TextNode` | Plain text (entities decoded) |
| `EmphasisNode` | Stressed emphasis (`<em>`) |
| `StrongNode` | Strong importance (`<strong>`) |
| `CodeSpanNode` | Inline code (`` `code` ``) |
| `LinkNode` | Hyperlink (destination always resolved) |
| `ImageNode` | Embedded image |
| `AutolinkNode` | Bare URL or email autolink |
| `RawInlineNode` | Verbatim inline content for a specific back-end |
| `HardBreakNode` | Forced line break (`<br />`) |
| `SoftBreakNode` | Soft newline within a paragraph |

## Key Design Decisions

**No `LinkDefinitionNode`** — links in the IR are always fully resolved.
Markdown's `[text][label]` reference syntax is resolved by the front-end;
the IR only ever contains `LinkNode { Destination: "…" }`.

**`RawBlockNode` / `RawInlineNode`** instead of `HtmlBlockNode` /
`HtmlInlineNode`. A `Format` field (`"html"`, `"latex"`, …) identifies the
target back-end. Renderers skip nodes with an unknown `Format`.

## Spec

TE00 — Document AST
