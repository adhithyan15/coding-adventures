# Changelog

All notable changes to `coding_adventures_gfm_parser` are documented here.

## [0.1.0] - 2026-03-24

### Added

- Initial release.
- Full GFM 0.31.2 compliance — passes all 652 specification examples.
- Two-phase parsing architecture:
  - Phase 1 (block structure): headings (ATX + setext), paragraphs, fenced code
    blocks, indented code blocks, blockquotes, ordered and unordered lists,
    thematic breaks, HTML blocks (types 1-7), link reference definitions.
  - Phase 2 (inline content): emphasis, strong, code spans, links (inline,
    full reference, collapsed reference, shortcut reference), images, autolinks,
    raw inline HTML, backslash escapes, entity/numeric character references,
    hard breaks, soft breaks.
- Delimiter stack algorithm (GFM Appendix A) for emphasis/strong.
- Full Unicode support: `unicode_whitespace?`, `unicode_punctuation?` per spec.
- 1698 named HTML character entities.
- Tab-expansion to 4-space tab stops.
- State-machine driven HTML block and fenced code block continuation.
- 678 tests (25 basic smoke tests + 653 GFM spec examples).
- 92%+ line coverage, 87%+ branch coverage.
- Spec: TE01 — GFM Parser.

### Fixed

- `block_detect_loop = true; break` pattern changed to `next` so the block
  detection loop correctly restarts after creating a list item or blockquote —
  this fixed list item content and blockquote content parsing.
- `&quot;` entity in `entities.rb` corrected to `"\""` (was incorrectly `"\\"`).
- `Scanner#match` fixed to use `@source[@pos, str.length] == str` instead of
  the invalid `String#start_with?(str, integer_offset)` Ruby API.
- Removed infinite-recursion delegate methods (`normalize_link_label`,
  `normalize_url`, `ascii_punctuation?`, etc.) that called themselves via
  `CommonmarkParser.method_name`.
