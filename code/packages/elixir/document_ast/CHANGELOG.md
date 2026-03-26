# Changelog — CodingAdventures.DocumentAst

## [0.1.0] — 2026-03-24

### Added

- Initial Elixir port of the Document AST format-agnostic IR
- All block node constructor functions: `document/1`, `heading/2`, `paragraph/1`, `code_block/2`, `blockquote/1`, `list/4`, `list_item/1`, `thematic_break/0`, `raw_block/2`
- All inline node constructor functions: `text/1`, `emphasis/1`, `strong/1`, `code_span/1`, `link/3`, `image/3`, `autolink/2`, `raw_inline/2`, `hard_break/0`, `soft_break/0`
- Full `@type` specs for all node types
- Comprehensive ExUnit test suite (>95% coverage)
- Literate `@moduledoc` and `@doc` documentation with examples
