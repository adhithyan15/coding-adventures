# Changelog — markdown-canvas-demo

## Unreleased

### Added

- Initial implementation. Parses CommonMark, lays out via `layout-block` with
  `CanvasTextMeasurer`, emits `PaintText` via `layout-to-paint` with
  `textEmitMode: "text"`, and paints into a `<canvas>` through
  `paint-vm-canvas`. Zero DOM for output.
- Live re-render on every keystroke; stats overlay shows instruction count,
  PaintText count, and frame time.
- Small normalization helper that coerces `LayoutNode.width: null` →
  `size_fill()` and `LayoutNode.height: null` → `size_wrap()` so the demo
  can run against the current document-ast-to-layout output (which emits
  null sizes by default).
