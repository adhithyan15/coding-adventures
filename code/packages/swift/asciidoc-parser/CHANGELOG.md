# Changelog — AsciidocParser (Swift)

All notable changes to this package will be documented here.

## [1.0.0] — 2026-04-04

### Added

- Initial release of the Swift AsciiDoc parser.
- `parse(_:)` public entry point chains `BlockParser` and `InlineParser`.
- **BlockParser** (Phase 1): line-by-line state machine supporting:
  - Section headings `=` through `======` (levels 1–6)
  - Thematic breaks (`'''`)
  - Single-line comments (`//`) silently discarded
  - Attribute lists (`[source,lang]`) to set code block language
  - Listing (code) blocks fenced with `----`
  - Literal blocks fenced with `....` (language: nil)
  - Passthrough blocks fenced with `++++` → `RawBlockNode(format: "html")`
  - Quote blocks fenced with `____` → recursively parsed `BlockquoteNode`
  - Unordered lists (`* item`, `** item`, up to 6 levels)
  - Ordered lists (`. item`, `.. item`, up to 6 levels)
  - Nested list building via `buildNestedList(_:ordered:)`
  - Paragraphs (any other non-blank content)
- **InlineParser** (Phase 2): left-to-right character scanner supporting:
  - `**text**` → `StrongNode` (unconstrained strong)
  - `*text*` → `StrongNode` (constrained strong — AsciiDoc bold!)
  - `__text__` → `EmphasisNode` (unconstrained emphasis)
  - `_text_` → `EmphasisNode` (constrained emphasis)
  - `` `code` `` → `CodeSpanNode` (verbatim, no inner markup)
  - `link:url[label]` → `LinkNode`
  - `image:url[alt]` → `ImageNode`
  - `<<anchor,text>>` and `<<anchor>>` → `LinkNode` (cross-references)
  - `https://url` and `http://url` → `AutolinkNode` or `LinkNode`
  - Soft breaks (single `\n`) and hard breaks (`  \n`, `\\\n`)
- 30+ unit tests in `ParserTests.swift` covering all block and inline constructs.
