# Changelog

## [0.1.0] - 2026-04-04

### Added
- Initial release of the AsciiDoc parser for Go.
- Two-phase parsing architecture: block parser (state machine) + inline parser.
- Block-level support: headings (= through ======), paragraphs, thematic breaks ('''), fenced code blocks (----), literal blocks (....), passthrough blocks (++++), quote blocks (____), unordered lists (* **), ordered lists (. ..), and comments (//).
- `[source,lang]` attribute for language-tagged code blocks.
- Inline support: strong (*text*, **text**), emphasis (_text_, __text__), code spans (`code`), links (link:url[text]), images (image:url[alt]), cross-references (<<anchor,text>>), bare URLs (autolinks), URLs with bracket labels, hard breaks (two trailing spaces, backslash-newline), soft breaks (plain newline).
- Note: AsciiDoc *text* = strong (not emphasis) — opposite of Markdown.
- Produces DocumentNode output compatible with document-ast-to-html and all other Document AST back-ends.
- 30+ unit tests covering block and inline parsing.
