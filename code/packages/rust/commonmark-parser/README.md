# commonmark-parser

CommonMark 0.31.2 compliant Markdown parser — produces a Document AST.

## Compliance

652/652 (100%) of the CommonMark 0.31.2 specification examples pass.

## What it does

Parses a CommonMark Markdown string into a `DocumentNode` AST — the format-agnostic IR defined in the `document-ast` crate.

```rust
use commonmark_parser::parse;

let doc = parse("# Hello\n\nWorld *with* emphasis.\n");
assert_eq!(doc.children.len(), 2); // heading + paragraph
```

## Two-phase parsing

The parser works in two phases:

```text
Phase 1 — Block structure
  Input: raw Markdown text
  Output: Vec<FinalBlock> + LinkRefMap

  Processes: headings, paragraphs, code blocks (fenced and indented),
             blockquotes, lists, HTML blocks, link reference definitions,
             thematic breaks

Phase 2 — Inline content
  Input: Vec<FinalBlock> + LinkRefMap
  Output: DocumentNode

  Processes: emphasis and strong (*/_), links, images, autolinks,
             code spans, raw HTML, backslash escapes, character entities,
             hard/soft line breaks
```

## Features

- **Full spec compliance**: all 652 CommonMark 0.31.2 examples pass
- **Correct tab handling**: virtual column tracking for partial-tab stripping
- **Delimiter stack**: emphasis resolution per CommonMark Appendix A
- **2125 HTML5 entities**: binary-searched static lookup table
- **Unicode support**: proper punctuation and whitespace classification for emphasis flanking rules
- **Link reference definitions**: multi-line titles, angle-bracket destinations, proper normalization

## Usage

```rust
use commonmark_parser::parse;

// Parse any CommonMark Markdown
let doc = parse("## Heading\n\n- item 1\n- item 2\n");
assert_eq!(doc.children.len(), 2); // heading + list
```

## How it fits in the stack

```text
document-ast           ← format-agnostic types
      ↓ types
commonmark-parser      ← you are here
parse(markdown) → DocumentNode
      ↓
document-ast-to-html or any other renderer
```

## Version

0.1.0
