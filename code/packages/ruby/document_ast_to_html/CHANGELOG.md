# Changelog

All notable changes to `coding_adventures_document_ast_to_html` are documented here.

## [0.1.0] - 2026-03-24

### Added

- Initial release.
- Full HTML renderer for all 19 Document AST node types (9 block, 10 inline).
- `CodingAdventures::DocumentAstToHtml.to_html(document, sanitize: false)`
  public entry point.
- Tight vs loose list rendering: suppresses `<p>` wrappers inside tight
  `ListNode` children (CommonMark tight-list rule).
- `normalize_url` — percent-encodes characters outside the RFC 3986 unreserved
  + reserved set so autolink and link URLs are safe for HTML attributes.
- `sanitize_url` — blocks dangerous URL schemes: `javascript:`, `vbscript:`,
  `data:`, `blob:`.
- `escape_html` — HTML-escapes `&`, `<`, `>`, `"`, `'` in text content and
  attribute values.
- `sanitize: true` mode strips all `RawBlockNode` and `RawInlineNode` output
  for rendering untrusted user-supplied Markdown.
- `render_autolink` calls `normalize_url` on non-email destinations to
  percent-encode characters such as `[`, `]`, `` ` ``, `\` (required for
  CommonMark spec examples 20, 346, 526, 538, 603).
- Spec: TE02 — Document AST to HTML.
- 47 unit tests; 92%+ line coverage.
