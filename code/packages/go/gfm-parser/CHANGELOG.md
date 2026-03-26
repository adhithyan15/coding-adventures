# Changelog — commonmark-parser (Go)

## [0.1.0] — 2026-03-24

### Added

- Initial Go port of the GFM 0.31.2 Markdown block and inline parser
  (translated from the TypeScript reference implementation).
- **Phase 1 — block parser** (`block_parser.go`):
  - Tab-aware line processing with virtual-column tracking for 4-column tab stops.
  - Container stack driven by `mutableDocument`, `mutableBlockquote`,
    `mutableList`, and `mutableListItem` types.
  - Leaf block types: `mutableParagraph`, `mutableFencedCode`,
    `mutableIndentedCode`, `mutableHtmlBlock`, `mutableHeading`,
    `mutableThematicBreak`, `mutableLinkDef`.
  - HTML block types 1–7 with their distinct opening/closing conditions.
  - Setext and ATX heading detection.
  - Link reference definition collection (resolved during Phase 1, not emitted
    in the AST).
  - Tight/loose list determination via blank-line tracking.
  - Lazy paragraph continuation for blockquotes and list items.
- **Phase 2 — inline parser** (`inline_parser.go`):
  - GFM Appendix A delimiter-stack algorithm for emphasis and strong
    emphasis resolution.
  - Code spans with backtick normalization.
  - Inline HTML passthrough.
  - Autolinks (`<url>` and `<email>` forms).
  - Inline links and reference links (full, collapsed, shortcut).
  - Images (`![](...)`).
  - Hard and soft line breaks.
- **Scanner** (`scanner.go`): position-based cursor over a UTF-8 string with
  helpers for character classification (ASCII/Unicode whitespace, ASCII/Unicode
  punctuation), `NormalizeLinkLabel`, `NormalizeURL`, and tab-stop arithmetic.
- **Entity decoding** (`entities.go`, `entities_table.go`): full HTML5
  named-entity table (2 125 entries), `DecodeEntity`, `DecodeEntities`, and
  `EscapeHTML` functions.
- **`Parse` entry point** (`parser.go`): wires Phase 1 → Phase 2 and returns a
  `*documentast.DocumentNode`.
- All 652 GFM 0.31.2 spec examples pass (verified by the `commonmark`
  package's `TestGFMSpec`).
