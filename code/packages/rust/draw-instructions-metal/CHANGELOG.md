# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-01

### Added

- `MetalRenderer` implementing `Renderer<PixelBuffer>` trait
- `render_metal()` convenience function
- MSL shaders for solid-color rectangles and textured text quads
- CoreText-based text rasterization
- Support for all V2 draw instructions (Rect, Text, Group, Line, Clip)
- Hex color parsing (#rgb, #rrggbb, #rrggbbaa)
