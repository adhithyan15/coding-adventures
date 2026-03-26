# CodingAdventures.CommonmarkParser

GFM 0.31.2 parser for Elixir — 100% spec conformity (652/652 examples).

Converts a GitHub Flavored Markdown string into a `CodingAdventures.DocumentAst` node tree
using a two-phase pipeline: block structure parsing followed by inline content parsing.

Spec: TE00 — Document AST

## How it fits in the stack

```
GFM text
     │
     ▼
CommonmarkParser.BlockParser   ← Phase 1: splits lines into block tree
     │                           (headings, lists, blockquotes, code blocks, …)
     ▼
CommonmarkParser.InlineParser  ← Phase 2: parses inline content per block
     │                           (emphasis, links, images, code spans, …)
     ▼
CodingAdventures.DocumentAst   ← format-agnostic IR
     │
     ▼
DocumentAstToHtml (or any renderer)
```

## Usage

```elixir
alias CodingAdventures.CommonmarkParser

{doc, refs} = CommonmarkParser.parse("# Hello\n\nWorld!\n")
# doc is a %{type: :document, children: [...]} tree
```

The returned document tree uses the `CodingAdventures.DocumentAst` node types.
Pass it to `CodingAdventures.DocumentAstToHtml.render/1` to produce HTML.

## Design

### Phase 1 — Block Parser

The block parser walks the input line by line. It maintains a stack of open
container blocks (document → blockquote → list → list_item) and detects new
leaf blocks (paragraphs, headings, code blocks, thematic breaks, HTML blocks).

Key rules implemented:
- Lazy paragraph continuation (GFM §5.1, §5.3)
- Tab expansion (4-column tab stops)
- Tight vs loose list detection
- Link reference definition extraction
- Setext and ATX headings
- Fenced and indented code blocks
- Seven HTML block types

### Phase 2 — Inline Parser

The inline parser implements the GFM delimiter stack algorithm (Appendix A).
It processes each block's raw content string into an inline node tree.

Key rules implemented:
- Emphasis and strong (*, _, flanking rules, rule of 3)
- Links and images (inline, full ref, collapsed ref, shortcut ref)
- Nested link/image handling with dead-opener deactivation
- Code spans (backtick matching, normalization)
- Autolinks (URL and email)
- Raw HTML (tags, comments, processing instructions, CDATA, declarations)
- Hard and soft line breaks
- Backslash escapes
- HTML entity references (named, decimal, hex)

## Running Tests

```sh
mix deps.get
mix test
```

The test suite runs all 652 examples from the GFM 0.31.2 specification
(`test/fixtures/spec.json`).
