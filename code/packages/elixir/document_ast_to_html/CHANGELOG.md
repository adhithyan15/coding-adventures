# Changelog — CodingAdventures.DocumentAstToHtml

All notable changes to this package are documented here.

## [0.1.0] — 2026-03-24

Initial release. Implements the HTML rendering back-end for the Document AST.

### Features

- Renders all Document AST block node types: document, heading (h1–h6),
  paragraph, code_block, blockquote, list (ordered/unordered), list_item,
  thematic_break, raw_block
- Renders all Document AST inline node types: text, emphasis, strong, code_span,
  link, image, autolink (URL and email), raw_inline, hard_break, soft_break
- Tight list rendering: suppresses `<p>` wrappers around list item paragraph
  content; handles mixed tight items correctly
- Heading level clamping to 1–6
- Code block language class: `<code class="language-X">` for fenced blocks
  with info string (first word of info string only)
- URL escaping: `&` in href/src attributes escaped to `&amp;`; other characters
  are left as-is (parser pre-normalizes them)
- HTML escaping: `&`, `<`, `>`, `"` escaped in text and attribute values
- Autolink email href uses `mailto:` prefix
- Autolink URL href uses `normalize_url_for_html` for percent-encoding of
  characters not in the safe ASCII URL set
