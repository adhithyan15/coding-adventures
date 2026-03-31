# Changelog

## 0.1.0 — 2026-03-31

### Added
- Initial release of the ASCII/Unicode text renderer for draw instructions.
- Stroked rectangles rendered as box-drawing character outlines.
- Filled rectangles rendered as solid block characters.
- Horizontal and vertical lines with endpoint-aware direction flags.
- Diagonal line approximation using Bresenham's algorithm.
- Intersection merging via direction bitmask for correct junction characters.
- Text rendering with start/middle/end alignment.
- Clip support for constraining children to rectangular regions.
- Group recursion for nested instruction trees.
- Configurable scale (default 8px/col, 16px/row).
- Trailing whitespace trimming per line.
- Implements the `DrawInstructions` behaviour for use with `render_with/2`.
