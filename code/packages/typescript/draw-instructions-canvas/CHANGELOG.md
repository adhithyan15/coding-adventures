# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-01

### Added

- Canvas renderer for generic draw instructions
- `renderCanvas(scene, ctx)` convenience function
- `createCanvasRenderer(ctx)` factory implementing `DrawRenderer<void>`
- Support for all five instruction types: rect, text, line, group, clip
- Alignment mapping: `"middle"` → Canvas `"center"` (avoids silent misalignment)
- Font-weight support in text instructions
- Nested clip support via `save/beginPath/clip/restore` idiom
