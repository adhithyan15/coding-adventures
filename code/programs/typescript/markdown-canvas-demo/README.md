# markdown-canvas-demo

Live Markdown → Canvas demo. Type Markdown on the left, watch it render
directly into an HTML canvas on the right — **no DOM for output**. Every
pixel of the rendered document lives inside a single `<canvas>` element.

## What it does

- Split-pane UI: Markdown editor on the left, canvas renderer on the right.
- Re-renders the canvas on every keystroke.
- Shows instruction counts and frame time in the footer.
- Demonstrates the full TXT03d pipeline:
  - Parse CommonMark → Document AST
  - Document AST → LayoutNode
  - `layout-block` + `CanvasTextMeasurer` → PositionedNode
  - `layout-to-paint` with `textEmitMode: "text"` → PaintScene of `PaintText`
    instructions
  - `paint-vm-canvas` dispatches each `PaintText` via `ctx.fillText`

## Why canvas, not DOM?

Canvas 2D is an imperative rasterizer — no retained tree, no layout engine
of its own, no HTML parsing. That makes it the right target for fully
custom layout, animations, very large documents, and offscreen / worker
rendering. But canvas has no `drawGlyphs(glyph_ids[])` API — it can only
draw strings via `ctx.fillText`. That constraint is what `PaintText`
(spec P2D00 amendment + TXT03d) encodes: when the paint backend does its
own shaping internally, the IR speaks in strings, not glyph indices.

## Running locally

```bash
cd code/programs/typescript/markdown-canvas-demo
npm install
npm run dev
```

Then open http://localhost:5173/.

## Related packages

- [`@coding-adventures/paint-instructions`](../../packages/typescript/paint-instructions) — the `PaintText` instruction is defined here.
- [`@coding-adventures/paint-vm-canvas`](../../packages/typescript/paint-vm-canvas) — the canvas-side handler.
- [`@coding-adventures/layout-to-paint`](../../packages/typescript/layout-to-paint) — emits `PaintText` when `textEmitMode: "text"` is set.
- [`@coding-adventures/layout-text-measure-canvas`](../../packages/typescript/layout-text-measure-canvas) — measures text via `ctx.measureText` for layout.

## Spec

- [TXT03d — Canvas Text Backend](../../../specs/TXT03d-canvas-text-backend.md)
- [P2D00 — Paint Instructions](../../../specs/P2D00-paint-instructions.md) (see the "GlyphRun vs Text" section)
