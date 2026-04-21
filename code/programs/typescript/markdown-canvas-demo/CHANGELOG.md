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
  `size_fill()` (block) or `size_wrap()` (inline) and `LayoutNode.height:
  null` → `size_wrap()` so the demo can run against the current
  document-ast-to-layout output (which emits null sizes by default).
- List-item adapter: flex-row containers from `document-ast-to-layout` are
  flattened into an inline sequence of text leaves. Layout-block does not
  implement flex, so without this adapter "1." and the list body would
  stack vertically. A proper fix lives in a future layout-flexbox
  integration; this keeps the demo readable now.
