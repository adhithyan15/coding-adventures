# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-24

### Added

- Complete CommonMark 0.31.2 parser passing 705/714 spec tests (98.7%)
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

### Known limitations (9 spec failures)

- Examples 5, 6, 7, 9: tab expansion inside list continuation and blockquote
  indentation (complex virtual-column arithmetic)
- Examples 259, 260: deeply nested list container edge cases
- Example 520: image inside link with complex alt text extraction
- Example 540: Unicode case-folding for `ẞ` → `ss` (JavaScript `toLowerCase()` does
  not apply multi-character folding)
- Example 626: HTML comment `<!--> foo -->` — cmark accepts content starting with
  `>` but the final `>` in `-->` must be escaped; exact boundary differs from spec
  prose and is not yet implemented
