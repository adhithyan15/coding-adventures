# Changelog

All notable changes to the `coding_adventures_draw_instructions_text` Ruby gem.

## [0.1.0] - 2026-03-31

### Added

- Initial release of the ASCII/Unicode text renderer
- `DrawInstructionsText.render_text()` for scene-to-string conversion
- `DrawInstructionsText::TextRenderer` duck-typed renderer for use with `render_with`
- Configurable scale options (default: 8px/col, 16px/row)
- Box-drawing character output: corners, edges, tees, crosses
- Filled rectangle support using block characters
- Line rendering with endpoint-aware direction flags
- Intersection merging via direction bitmask for junction characters
- Text rendering with start/middle/end alignment
- Clip region support (intersects with parent clip bounds)
- Group recursion
- Trailing whitespace trimming per line
