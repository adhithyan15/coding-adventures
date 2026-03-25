# Changelog — commonmark-parser

## 0.1.0 — 2026-03-24

Initial release.

### Added

- Two-phase CommonMark 0.31.2 parser: block structure (Phase 1) + inline content (Phase 2)
- **Block parser** (`block_parser.rs`): node arena design, handles all CommonMark block constructs:
  - ATX and setext headings
  - Fenced and indented code blocks
  - Blockquotes with lazy continuation
  - Ordered and unordered lists with tight/loose detection
  - HTML blocks (types 1–7)
  - Link reference definitions (multi-line, all title quote styles)
  - Thematic breaks
  - Partial-tab stripping with virtual column tracking
- **Inline parser** (`inline_parser.rs`): delimiter stack algorithm (CommonMark Appendix A):
  - Emphasis and strong emphasis (`*` / `_`)
  - Links and images with inline, reference, and collapsed forms
  - Code spans with backtick-length matching and whitespace normalization
  - Autolinks (URL and email)
  - Raw HTML inlines
  - Backslash escapes
  - Character references (named, decimal, hex)
  - Hard and soft line breaks
- **Entity table** (`entities_table.rs`): 2125 HTML5 named character references, sorted for binary search
- **Scanner** (`scanner.rs`): cursor-based scanner with Unicode punctuation and whitespace classification
- 652/652 (100%) CommonMark 0.31.2 spec examples pass
- Unit tests and doctests

### Key design decisions

- **Node arena pattern**: avoids Rust borrow-checker issues with mutable recursive trees by storing all nodes in a flat `Vec` indexed by `NodeId`
- **`unicode-general-category` crate**: used for Unicode P* and S* category lookup (emphasis flanking rules)
- **`serde` + `serde_json` dev-dependencies**: for loading the CommonMark spec JSON in integration tests
