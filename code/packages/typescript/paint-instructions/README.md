# @coding-adventures/paint-instructions

Universal 2D paint intermediate representation (IR). Part of the P2D series.

## What it is

`paint-instructions` defines the complete type system for the PaintInstructions IR (P2D00). It is the shared vocabulary between **producers** (chart builders, diagram renderers, game engines) and **backends** (Canvas, SVG, Metal, terminal).

```
Producer (chart, barcode, mermaid diagram)
  → PaintScene / PaintInstruction[]     ← this package
  → PaintVM (paint-vm)
  → Backend (paint-vm-svg, paint-vm-canvas, paint-vm-metal, ...)
```

This package is **pure types + builder helpers**. Zero runtime dependencies. No rendering code.

## Instruction types

| Kind | Description |
|---|---|
| `rect` | Filled/stroked rectangle with optional corner radius |
| `ellipse` | Filled/stroked ellipse or circle |
| `path` | Arbitrary vector path (line_to, cubic_to, arc_to, ...) |
| `glyph_run` | Pre-positioned glyphs from a font (shaped by layout layer) |
| `group` | Logical container for transform/opacity inheritance |
| `layer` | Offscreen compositing surface with filters and blend modes |
| `line` | Single straight line segment |
| `clip` | Rectangular clip mask for children |
| `gradient` | Linear or radial colour gradient, referenced by id from fill fields |
| `image` | Raster image from URI string or `PixelContainer` |

## Usage

```typescript
import {
  paintScene, paintRect, paintEllipse, paintLayer,
  type PaintScene,
} from "@coding-adventures/paint-instructions";

const scene: PaintScene = paintScene(800, 600, "#ffffff", [
  paintRect(20, 20, 760, 560, { fill: "#f8fafc", stroke: "#e2e8f0", corner_radius: 8 }),
  paintEllipse(400, 300, 100, 100, { fill: "#3b82f6" }),
  paintLayer([
    paintEllipse(200, 200, 80, 80, { fill: "#ef4444" }),
  ], {
    filters: [{ kind: "blur", radius: 12 }],
    blend_mode: "screen",
  }),
]);

// Pass to a backend:
// import { createCanvasVM } from "@coding-adventures/paint-vm-canvas";
// const vm = createCanvasVM();
// vm.execute(scene, ctx);
```

## Composable pipeline

Every type in this package is designed to snap into a pipeline:

```
PaintScene ──▶ PaintVM.execute()  ──▶ rendered output
PaintScene ──▶ PaintVM.export()   ──▶ PixelContainer
PixelContainer ──▶ ImageCodec.encode() ──▶ .png / .webp / .jpg bytes
PixelContainer ──▶ PaintImage.src ──▶ embedded in another scene
```

## PaintLayer vs PaintGroup

| | PaintGroup | PaintLayer |
|---|---|---|
| Offscreen buffer | No — renders to parent | Yes — separate buffer |
| Filters | Not supported | Supported |
| Blend modes | Not supported | Supported |
| Performance | Fast | Costs one offscreen allocation |

Use `PaintGroup` for transforms and logical grouping. Use `PaintLayer` when you need blur, drop shadow, or blend modes.

## Stack position

This package is in the `P2D` series. It is the foundational shared contract for the entire 2D rendering pipeline:

| Spec | Package | Description |
|---|---|---|
| P2D00 | `paint-instructions` | IR types — **this package** |
| P2D01 | `paint-vm` | Dispatch-table VM (execute + patch + export) |
| P2D02 | `paint-vm-svg` | SVG backend |
| P2D03 | `paint-vm-canvas` | HTML5 Canvas backend |
| P2D04 | `paint-vm-terminal` | Terminal/ASCII backend |
| P2D05 | `paint-vm-metal` | Apple Metal backend (Rust FFI) |
