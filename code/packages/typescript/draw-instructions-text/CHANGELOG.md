# Changelog

## 0.1.0 — 2026-03-31

### Added

- ASCII/Unicode text renderer for draw-instructions scenes
- Stroked rectangles render as box-drawing characters (┌ ┐ └ ┘ ─ │)
- Filled rectangles render as block characters (█)
- Horizontal and vertical lines with proper intersection handling (┼ ┬ ┴ ├ ┤)
- Text rendering with start/middle/end alignment
- Clip instruction support (constrains drawing to rectangular region)
- Group instruction support (recurses into children)
- Configurable scale factor (scaleX, scaleY) for pixel-to-character mapping
- Intersection tag buffer for correct junction character resolution
- `TEXT_RENDERER` default renderer, `renderText()` convenience function, `createTextRenderer()` factory
- Spec: `code/specs/draw-instructions-text.md`
