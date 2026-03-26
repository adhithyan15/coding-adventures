# Changelog — CodingAdventures.Commonmark

All notable changes to this package are documented here.

## [0.1.0] — 2026-03-24

Initial release. Thin pipeline wrapper combining `CommonmarkParser` and
`DocumentAstToHtml` into a single `to_html/1` function.

### Features

- `to_html/1` — converts a CommonMark string to an HTML string in one call
- Full CommonMark 0.31.2 conformity via the underlying `CommonmarkParser`
  (652/652 spec examples)
- All block and inline types supported: headings, paragraphs, lists, blockquotes,
  code blocks, thematic breaks, emphasis, strong, links, images, code spans,
  autolinks, raw HTML, and more
