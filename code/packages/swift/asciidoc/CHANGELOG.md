# Changelog — Asciidoc (Swift)

All notable changes to this package will be documented here.

## [1.0.0] — 2026-04-04

### Added

- Initial release of the Swift AsciiDoc → HTML convenience wrapper.
- `toHtml(_:)` public function chains `AsciidocParser.parse(_:)` and
  `DocumentAstToHtml.render(_:)` for one-call AsciiDoc-to-HTML conversion.
- 20+ end-to-end `toHtml` tests covering:
  - All 6 heading levels (`=` through `======`)
  - Thematic break (`'''` → `<hr />`)
  - Code block with language (`[source,swift]` + `----`)
  - Code block without language
  - Literal block (`....`)
  - Passthrough block (`++++` → raw HTML verbatim)
  - Quote block (`____` → `<blockquote>`)
  - Unordered list (`* item` → `<ul>/<li>`)
  - Ordered list (`. item` → `<ol>/<li>`)
  - Single and multiple paragraphs
  - Bold text (`*bold*` → `<strong>` — AsciiDoc semantics!)
  - Italic text (`_italic_` → `<em>`)
  - Inline code (`` `code` `` → `<code>`)
  - Link macro (`link:url[text]` → `<a href>`)
  - Image macro (`image:url[alt]` → `<img>`)
  - Cross-reference (`<<id,text>>` → `<a href="#id">`)
  - Bare URL autolinks (`https://…` → `<a href>`)
  - HTML escaping in paragraph content
  - Empty input
