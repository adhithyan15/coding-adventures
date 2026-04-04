# Changelog

All notable changes to `@coding-adventures/asciidoc` will be documented here.

## 0.1.0 — Initial release

- `toHtml(text: string): string` convenience function combining parse() + render().
- Re-exports `parse` from @coding-adventures/asciidoc-parser.
- Re-exports `render` from @coding-adventures/document-ast-to-html.
- Re-exports all Document AST types for users who need them.
- 10+ end-to-end tests verifying headings, paragraphs, strong/emphasis,
  code spans, code blocks with language hints, lists, blockquotes, thematic
  breaks, and passthrough blocks.
