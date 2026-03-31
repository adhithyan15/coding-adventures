# Changelog

## 0.1.0 - 2026-03-31

### Added

- Initial release of the ASCII/Unicode text renderer.
- `TextRenderer` class implementing `DrawRenderer[str]` protocol.
- `render_text()` convenience function.
- `TEXT_RENDERER` pre-configured singleton.
- Stroked rectangles rendered as box-drawing outlines.
- Filled rectangles rendered as block characters.
- Horizontal, vertical, and diagonal line support.
- Intersection merging via direction bitmask for junction characters.
- Text rendering with start/middle/end alignment.
- Clip region support with nested intersection.
- Group instruction recursion.
- Configurable pixel-to-character scale (default 8 px/col, 16 px/row).
