# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] — 2026-03-24

### Changed

- **Package split** (spec TE00 — Document AST): the monolithic package has been
  split into three constituent packages connected by the format-agnostic
  Document AST IR:
  - `@coding-adventures/document-ast` — pure type definitions (no runtime code)
  - `@coding-adventures/commonmark-parser` — Markdown → DocumentNode
  - `@coding-adventures/document-ast-to-html` — DocumentNode → HTML string
- `@coding-adventures/commonmark` is now a **thin pipeline convenience package**
  that re-exports `parse` and `toHtml` from the two constituent packages.
- `toHtml` is now part of the public API of this package (previously unexported).
- All AST types are now re-exported from `@coding-adventures/document-ast`.
- `HtmlBlockNode { type: "html_block" }` → `RawBlockNode { type: "raw_block"; format: "html" }`
- `HtmlInlineNode { type: "html_inline" }` → `RawInlineNode { type: "raw_inline"; format: "html" }`
- `LinkDefinitionNode` removed from public types — link references are fully
  resolved before the IR is produced and never appear in the output tree.
- Version bumped to 0.2.0.

### Preserved

- All 652 CommonMark 0.31.2 spec examples still pass (100% conformity).
- Public API: `parse(markdown: string): DocumentNode` and `toHtml(doc: DocumentNode): string`.

---

## [0.1.0] - 2026-03-24

### Added

- Complete CommonMark 0.31.2 parser — **all 652 spec examples pass (100% conformity)**
- Block parser: ATX/setext headings, fenced/indented code blocks, blockquotes,
  ordered/unordered lists (tight/loose), thematic breaks, HTML blocks (types 1–7),
  link reference definitions, paragraphs
- Inline parser: emphasis/strong (delimiter-stack algorithm), links (inline,
  full-reference, collapsed, shortcut), images, code spans, autolinks, HTML inline
  (open/close tags, comments, processing instructions, CDATA, declarations),
  backslash escapes, entity/numeric character references, hard/soft breaks
- HTML renderer: AST → spec-compliant HTML string with correct tight-list handling
- `Scanner` utility: cursor-based string scanner with tab-expansion, sticky-regex
  matching, and Unicode character classification helpers
- Entity decoder: full HTML5 named entities table plus numeric character references
  (decimal up to 7 digits, hex up to 6 digits)
- URL normalization: percent-encodes characters unsafe in HTML href/src attributes
  including `[`, `]`
- Link label normalization: strip/collapse whitespace, Unicode case-fold (per §4.7),
  no backslash escaping applied to labels
- Correct raw-source label extraction for bracket matching (avoids escape-processing
  the label content before normalization)

### Fixed (achieving 100% spec conformity)

- **Tab expansion in lists/blockquotes (ex5, 6, 7, 9)**: Rewrote `indentOf` and
  `stripIndent` to accept a `baseCol` parameter (virtual column of the first
  character of the input string). This correctly handles partial-tab stripping:
  when a tab spans the strip boundary, the tab is consumed and leftover virtual
  spaces are prepended to the result. Also updated the blockquote `>` detector in
  block detection (step 6) and the list-item separator handler to use the same
  tab-aware arithmetic, threading `lineBaseCol` through all `continue blockDetect`
  paths.
- **Blank-line propagation in nested containers (ex259, 260)**: When a line like
  `>>` (double blockquote marker with no content) is stripped of container markers,
  the resulting empty line is treated as `effectiveBlank` for list-item continuation
  purposes, even if the raw line was not blank. This correctly makes the containing
  list item "see" the blank separator.
- **Deactivated bracket blocking image alt (ex520)**: The `]` handler in the inline
  parser now checks whether the top of the bracket stack is a deactivated non-image
  opener. If so, that `]` is its matching bracket — it is emitted as literal `]`
  and the opener is removed from the stack, preventing the search from
  over-scanning to find an outer `![` image opener.
- **Unicode Full Case Folding for ẞ (ex540)**: `normalizeLinkLabel` now applies
  `.replace(/ß/g, "ss")` after `toLowerCase()`, since JavaScript's `toLowerCase`
  maps `ẞ → ß` but not `ß → ss`.
- **HTML comment partial construct (ex626)**: The inline HTML comment parser now
  detects when `<!--` is followed by an invalid starter (`>` or `->`), and emits
  the partial construct as a raw `html_inline` node rather than escaping `<` as
  `&lt;`. This matches cmark's reference behaviour.
