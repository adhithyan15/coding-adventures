# commonmark-parser (Go)

A CommonMark 0.31.2-compliant Markdown parser in Go. Converts a Markdown string
into a `DocumentNode` abstract syntax tree (Document AST, spec TE00).

## Architecture

Parsing is a two-phase process modelled on the CommonMark reference implementation:

1. **Phase 1 — Block structure** (`block_parser.go`): reads the input line by
   line, building a mutable intermediate tree of block containers and leaf
   blocks (headings, paragraphs, code blocks, lists, blockquotes, HTML blocks,
   thematic breaks, and link reference definitions). Tab characters are expanded
   to 4-column tab stops using virtual-column arithmetic throughout.

2. **Phase 2 — Inline content** (`inline_parser.go`): walks each heading and
   paragraph node, runs the CommonMark delimiter-stack algorithm (Appendix A),
   and replaces raw text strings with the fully resolved inline node tree
   (`EmphasisNode`, `StrongNode`, `LinkNode`, `ImageNode`, `CodeSpanNode`, etc.).

Additional files:

- `scanner.go` — cursor-based scanner with character-classification helpers,
  Unicode whitespace/punctuation detection, link-label normalisation, and URL
  normalisation.
- `entities.go` / `entities_table.go` — HTML5 named-entity decoder (2 125
  entries) plus `DecodeEntities` and `EscapeHTML` functions.
- `parser.go` — thin `Parse` entry point that wires the two phases together.

## Usage

```go
import parser "github.com/adhithyan15/coding-adventures/code/packages/go/commonmark-parser"
import documentast "github.com/adhithyan15/coding-adventures/code/packages/go/document-ast"

// Parse Markdown → DocumentNode AST
doc := parser.Parse("# Hello\n\nWorld *with* emphasis.\n")
// doc.Children[0] is a HeadingNode{Level:1, ...}
// doc.Children[1] is a ParagraphNode{...}
```

For most use-cases prefer the higher-level `commonmark` package, which
combines this parser with the HTML renderer in a single import.

## Spec conformance

100% — all 652 CommonMark 0.31.2 spec examples pass (verified by the
`commonmark` package's `TestCommonMarkSpec`).

## Stack position

```
commonmark-parser   ← you are here
      ↓
  document-ast  (shared AST types)
```

The output is consumed by `document-ast-to-html` (or any other Document AST
back-end) to produce the final output format.

## Dependencies

- `document-ast` — shared AST node types (local replace directive in `go.mod`)

## Running tests

```
go test ./... -v -cover
```
