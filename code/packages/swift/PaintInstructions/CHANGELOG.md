# Changelog

## [Unreleased]

### Added

- `PaintLineInstruction`, `PaintGlyphPlacement`/`PaintGlyphRunInstruction`,
  `Transform2D`, `PaintGroupInstruction`, `PaintClipInstruction`,
  `PaintLayerInstruction` — bringing this package to the full
  `P2D02-paint-vm-ascii.md` instruction set (rect/line/glyph_run/group/clip/layer)
- `stroke`/`strokeWidth` fields on `PaintRectInstruction`
- `paintLine()`, `paintGlyphRun()`, `paintGroup()`, `paintClip()`, and
  `paintLayer()` builder helpers, matching the existing `paintRect()`
  convention

### Changed

- **Breaking (internal only — see below)**: `PaintInstruction` is now a
  proper closed sum type (`enum` with associated values, one case per
  instruction kind) instead of `typealias PaintInstruction =
  PaintRectInstruction`. Every real producer in this repository builds
  instructions exclusively through the `paintRect()`/`paintLine()`/etc.
  helper functions (never the concrete struct initializers directly), so
  this change is source-compatible for all of them. The two consumers that
  *did* dot-access rect fields directly on a `PaintInstruction`-typed value
  (`PaintVmMetalNative` and `PaintVmDirect2DNative`'s native rect-only
  renderers) were updated to `switch`/`if case .rect` instead — see their
  own CHANGELOGs.

This brings the Swift `PaintInstructions` package to parity with the
Kotlin/Java/Dart ports of the same package, and is the prerequisite for
building a Swift `paint-vm-ascii` backend and `cowsay` port (see
`code/specs/cowsay-paintvm-pipeline.md`).
