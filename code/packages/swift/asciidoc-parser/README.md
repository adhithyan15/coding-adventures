# AsciidocParser — Swift AsciiDoc Block + Inline Parser

Parses AsciiDoc source text into the format-agnostic [Document AST](../document-ast).

## Overview

This package is Phase 1 and Phase 2 of the AsciiDoc processing pipeline:

```
AsciiDoc text
     │
     ▼  BlockParser.parseBlocks(_:) — Phase 1: line-by-line block structure
[BlockNode]  (headings, paragraphs, code blocks, lists, …)
     │
     ▼  InlineParser.parse(_:) — Phase 2: character-level markup
BlockNode.document(DocumentNode(...))
```

The output is a `BlockNode.document(...)` that can be rendered to HTML using
[DocumentAstToHtml](../document-ast-to-html) or any other Document AST renderer.

## Usage

```swift
import AsciidocParser

let doc = parse("""
= My Document

Hello *world* and _everyone_.

== Section

[source,swift]
----
let x = 42
----
""")
// Returns a .document(DocumentNode(...)) with headings, paragraphs, code block
```

## AsciiDoc vs. CommonMark Differences

The key semantic difference from CommonMark:

| Syntax   | CommonMark          | AsciiDoc (this package) |
|----------|---------------------|-------------------------|
| `*text*` | EmphasisNode (italic) | **StrongNode (bold)**   |
| `_text_` | EmphasisNode (italic) | EmphasisNode (italic)   |
| `**t**`  | StrongNode (bold)   | StrongNode (bold)       |
| `__t__`  | StrongNode (bold)   | EmphasisNode (italic)   |

## Supported Block Constructs

| AsciiDoc Syntax | Output Node |
|-----------------|-------------|
| `= Title`       | `HeadingNode(level: 1)` |
| `== Section`    | `HeadingNode(level: 2)` … through `======` (level 6) |
| `'''`           | `ThematicBreakNode` |
| `[source,lang]` + `----…----` | `CodeBlockNode(language: "lang")` |
| `----…----`     | `CodeBlockNode(language: nil)` |
| `....…....`     | `CodeBlockNode(language: nil)` (literal block) |
| `++++…++++`     | `RawBlockNode(format: "html")` (passthrough) |
| `____…____`     | `BlockquoteNode` (quote block, recursively parsed) |
| `* item`        | `ListNode(ordered: false)` |
| `. item`        | `ListNode(ordered: true)` |
| `// comment`    | Silently discarded |

## Supported Inline Constructs

| AsciiDoc Syntax     | Output Node |
|---------------------|-------------|
| `**text**`          | `StrongNode` |
| `*text*`            | `StrongNode` (**bold** in AsciiDoc!) |
| `__text__`          | `EmphasisNode` |
| `_text_`            | `EmphasisNode` |
| `` `code` ``        | `CodeSpanNode` |
| `link:url[label]`   | `LinkNode` |
| `image:url[alt]`    | `ImageNode` |
| `<<id,text>>`       | `LinkNode` (cross-reference) |
| `https://url`       | `AutolinkNode` |
| `https://url[text]` | `LinkNode` |

## Dependency Chain

```
document-ast       (Layer 0) — shared IR
asciidoc-parser    (Layer 1) — this package
asciidoc           (Layer 2) — toHtml() convenience wrapper
```
