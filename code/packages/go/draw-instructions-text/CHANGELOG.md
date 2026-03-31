# Changelog

All notable changes to the `draw-instructions-text` Go package.

## [0.1.0] - 2026-03-31

### Added

- Initial release of the ASCII/Unicode text renderer
- `RenderText()` convenience function for scene-to-string conversion
- `NewTextRenderer()` for creating renderers with custom scale options
- `DefaultTextRenderer` with standard 8px/col, 16px/row scale
- `TextRendererOptions` for configuring pixel-to-character mapping
- Box-drawing character output: corners, edges, tees, crosses
- Filled rectangle support using block characters
- Line rendering with endpoint-aware direction flags
- Intersection merging via direction bitmask for junction characters
- Text rendering with start/middle/end alignment
- Clip region support (intersects with parent clip bounds)
- Group recursion
- Trailing whitespace trimming per line
- Implements `drawinstructions.Renderer[string]` interface
