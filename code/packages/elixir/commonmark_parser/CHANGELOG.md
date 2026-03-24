# Changelog — CodingAdventures.CommonmarkParser

All notable changes to this package are documented here.

## [0.1.0] — 2026-03-24

Initial release. Implements CommonMark 0.31.2 with 100% spec conformity (652/652 examples).

### Block Parser

- Full container block handling: blockquotes, lists (ordered and unordered), list items
- Tab expansion to 4-column tab stops via virtual-column arithmetic
- Lazy paragraph continuation for blockquotes and list items (no indentation limit)
- Tight vs loose list detection via blank-line tracking
- Link reference definitions extracted from paragraphs during finalization
- ATX headings (levels 1–6) with optional closing hashes
- Setext headings (levels 1–2) as underlined paragraphs
- Thematic breaks (---, ***, ___)
- Fenced code blocks (``` and ~~~, with info string)
- Indented code blocks (4-space indentation)
- Seven HTML block types per CommonMark §4.6
- Correct content_indent computation when list marker has 5+ spaces of padding
  (reduces content_indent so continuation lines work correctly, and prepends
  virtual spaces to initial content so indented code blocks are detected)

### Inline Parser

- Delimiter stack algorithm (CommonMark Appendix A)
- Emphasis (`*`, `_`) and strong with correct flanking and rule-of-3 checks
- Links and images: inline `(dest)`, full ref `[label]`, collapsed `[]`, shortcut
- Dead-opener deactivation: when a link forms, enclosing `[` openers are marked
  dead; subsequent `]` that would find a dead opener emit literal text instead
- Distinguishes failed-link openers (emit as text immediately) from
  deactivated openers (stay in acc as `{:opener_dead, text}` for dead-top check)
- Raw label text via byte-offset slicing for correct link label comparison
  (spec §4.7: backslash escapes are NOT applied for label matching)
- Full ref `[foo][bar]` returns nil when `bar` not found (no shortcut fallthrough)
- Code spans: backtick-matched, whitespace normalized
- Autolinks: URL and email
- Raw HTML: open/close tags, comments (with invalid-opener passthrough), processing
  instructions, CDATA, DOCTYPE declarations
- Hard breaks (trailing `  ` or `\`) and soft breaks (`\n`)
- Backslash escapes (ASCII punctuation only)
- HTML entity references: named (`&amp;`), decimal (`&#123;`), hex (`&#x7B;`)
- URL normalization: percent-encoding of unsafe characters
- Unicode link label normalization (trim, collapse whitespace, downcase)
