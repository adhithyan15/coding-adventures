# Changelog — cowsay-visualizer

## [0.1.0] — 2026-05-04

### Added

- Initial release — browser-native Cowsay rendered through PaintVM.
- **Canvas backend (primary)** — HiDPI-aware `<canvas>` output via
  `createCanvasVM().execute(scene, ctx)`. Calls `ctx.fillText()` for each
  cowsay line, delegating shaping to the browser's text stack.
- **SVG backend (toggle)** — vector SVG output via `renderToSvgString(scene)`.
  Injected inline so the output scales perfectly to any display size.
- **Backend-agnostic scene builder** — `buildCowsayScene(lines)` emits
  `PaintText` instructions with `canvas:` font_ref scheme. Both backends
  consume this scheme without modification.
- **Live re-render** — message textarea, backend radio buttons, and wrap-width
  field all trigger re-render on every input event.
- **Wrap-width control** — configurable line wrap (10–120 chars, default 40)
  preserves the classic 40-column cowsay aesthetic while supporting longer
  messages.
- **Speech bubble logic** — `cowsayLines()` word-wraps the message and formats
  single-line (`< msg >`) vs multi-line (`/ msg \`, `| msg |`, `\ msg /`)
  bubbles with correct ASCII corners.
