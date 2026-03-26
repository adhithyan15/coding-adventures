# coding_adventures_gfm_parser

GitHub Flavored Markdown parser that produces a Document AST.

## Overview

`coding_adventures_gfm_parser` parses GFM 0.31.2 Markdown into a
`DocumentNode` AST as defined by `coding_adventures_document_ast`. The result is a
format-agnostic IR ready for any back-end renderer (HTML, PDF, plain text, ...).

**Passes all 652 GFM 0.31.2 specification examples.**

## Two-Phase Architecture

The parse is two-phase, mirroring the GFM reference implementation:

**Phase 1 — Block structure** (`BlockParser`):
Walks the input line by line using a state machine, building a mutable tree of:
headings, paragraphs, fenced code blocks, blockquotes, lists, HTML blocks, etc.
Link reference definitions are collected and removed from the output.

**Phase 2 — Inline content** (inline parsing during AST conversion):
Each heading and paragraph's raw text is scanned for inline constructs:
emphasis, strong, code spans, links, images, autolinks, backslash escapes,
entity references, hard breaks, soft breaks.

The delimiter stack algorithm (GFM Appendix A) is used for emphasis/strong
matching with full left-flanking / right-flanking / mod-3 rule support.

## Installation

```ruby
gem "coding_adventures_gfm_parser"
```

## Usage

```ruby
require "coding_adventures_gfm_parser"

doc = CodingAdventures::CommonmarkParser.parse("# Hello\n\nWorld *with* emphasis.\n")

doc.type               # => "document"
doc.children[0].type   # => "heading"
doc.children[0].level  # => 1
doc.children[1].type   # => "paragraph"
```

## Spec

Implements spec **TE01 — GFM Parser** and depends on **TE00 — Document AST**.

## Requirements

- Ruby >= 3.2.0
- `coding_adventures_document_ast` ~> 0.1
- `coding_adventures_state_machine` ~> 0.1
