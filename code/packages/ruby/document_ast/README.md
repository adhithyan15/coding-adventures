# coding_adventures_document_ast

Format-agnostic Document AST (Intermediate Representation) for the coding-adventures stack.

## Overview

`coding_adventures_document_ast` defines the **Document AST** — a set of immutable value objects representing structured document content. It is the format-agnostic intermediate representation (IR) that connects front-end parsers (Markdown, AsciiDoc, HTML) to back-end renderers (HTML, PDF, plain text).

The design mirrors the role of LLVM IR in compiler pipelines: once any source format is parsed into a `DocumentNode` tree, any renderer that accepts a `DocumentNode` can produce output — without knowing anything about the original source format.

## Dependency Diagram

```
[Markdown source]  [AsciiDoc source]  [other source]
       ↓                  ↓                  ↓
  CommonmarkParser    (future)           (future)
       ↓                  ↓                  ↓
       └──────────────────┴──────────────────┘
                          ↓
               DocumentAst (this package)
                    DocumentNode tree
                          ↓
              ┌───────────┴───────────┐
              ↓                       ↓
       DocumentAstToHtml          (future: PDF, etc.)
          HTML string
```

## Node Types

All nodes are defined as Ruby `Data.define` objects — immutable frozen value objects (Ruby 3.2+).

### Block Nodes

| Type | Fields | Description |
|------|--------|-------------|
| `DocumentNode` | `children:` | Root of every document |
| `HeadingNode` | `level:`, `children:` | ATX (`# H1`) or setext heading |
| `ParagraphNode` | `children:` | Block of inline content |
| `CodeBlockNode` | `language:`, `value:` | Fenced or indented code |
| `BlockquoteNode` | `children:` | `>` blockquote |
| `ListNode` | `ordered:`, `start:`, `tight:`, `children:` | Ordered or unordered list |
| `ListItemNode` | `children:` | Single list item |
| `ThematicBreakNode` | — | `---` / `***` horizontal rule |
| `RawBlockNode` | `format:`, `value:` | Verbatim block (e.g. `format: "html"`) |

### Inline Nodes

| Type | Fields | Description |
|------|--------|-------------|
| `TextNode` | `value:` | Plain text |
| `EmphasisNode` | `children:` | `*em*` or `_em_` |
| `StrongNode` | `children:` | `**strong**` or `__strong__` |
| `CodeSpanNode` | `value:` | `` `code` `` |
| `LinkNode` | `destination:`, `title:`, `children:` | `[text](url)` |
| `ImageNode` | `destination:`, `alt:`, `title:` | `![alt](url)` |
| `AutolinkNode` | `destination:`, `is_email:` | `<url>` or `<email>` |
| `RawInlineNode` | `format:`, `value:` | Verbatim inline (e.g. `format: "html"`) |
| `HardBreakNode` | — | Two trailing spaces or `\` before newline |
| `SoftBreakNode` | — | Single newline within a paragraph |

## Installation

Add to your `Gemfile`:

```ruby
gem "coding_adventures_document_ast"
```

Or install directly:

```bash
gem install coding_adventures_document_ast
```

## Usage

```ruby
require "coding_adventures_document_ast"

include CodingAdventures::DocumentAst

# Build a document programmatically
doc = DocumentNode.new(children: [
  HeadingNode.new(level: 1, children: [
    TextNode.new(value: "Hello")
  ]),
  ParagraphNode.new(children: [
    TextNode.new(value: "World ")
    EmphasisNode.new(children: [TextNode.new(value: "with emphasis")])
  ])
])

doc.type          # => "document"
doc.children[0].type  # => "heading"
doc.children[0].level # => 1
```

## Spec

Implements spec **TE00 — Document AST**.

## Requirements

- Ruby >= 3.2.0 (uses `Data.define`)
