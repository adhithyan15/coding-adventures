# @coding-adventures/paint-vm-canvas

HTML5 Canvas backend for PaintVM (P2D03).

Renders a `PaintScene` to a `CanvasRenderingContext2D` using the imperative
Canvas 2D API. No DOM dependencies beyond the context itself — works in
browser main threads, Web Workers (via `OffscreenCanvas`), and server-side
runtimes that provide a Canvas implementation (e.g. `node-canvas` or Skia).

## Where it fits

```
PaintScene
    │
    ▼
PaintVM<CanvasRenderingContext2D>   ← this package
    │
    ├── execute(scene, ctx)         → pixels on canvas (immediate mode)
    ├── patch(prev, next, ctx)      → minimal repaint (retained mode)
    └── export(scene, opts?)        → PixelContainer (via OffscreenCanvas)
                                           │
                                           ▼
                                    paint-codec-png / paint-codec-webp
```

## Why Canvas over SVG for interactive rendering?

SVG is declarative and retained: the browser keeps a scene graph in memory.
For large scenes (thousands of bars in a chart, thousands of map tiles), that
overhead becomes a bottleneck.

Canvas is imperative: drawing commands go straight to the GPU rasterizer with
no retained tree. Better for:

- 60fps animations (call `execute()` every `requestAnimationFrame`)
- Large scenes (thousands of elements)
- Off-screen rendering in Web Workers
- Server-side PNG/WebP generation

## Installation

```sh
npm install @coding-adventures/paint-vm-canvas
```

## Usage

### Browser — draw to a canvas element

```typescript
import { createCanvasVM } from "@coding-adventures/paint-vm-canvas";
import { paintScene, paintRect, paintEllipse } from "@coding-adventures/paint-instructions";

const canvas = document.getElementById("chart") as HTMLCanvasElement;
const ctx = canvas.getContext("2d")!;

const vm = createCanvasVM();

vm.execute(
  paintScene(800, 400, "#ffffff", [
    paintRect(20, 20, 200, 100, { fill: "#3b82f6", corner_radius: 8 }),
    paintEllipse(400, 200, 80, 80, { fill: "#ef4444", stroke: "#b91c1c", stroke_width: 2 }),
  ]),
  ctx,
);
```

### OffscreenCanvas — Web Worker rendering

```typescript
// Inside a Web Worker
const offscreen = new OffscreenCanvas(800, 400);
const ctx = offscreen.getContext("2d")!;
vm.execute(scene, ctx);
const blob = await offscreen.convertToBlob({ type: "image/webp" });
postMessage(blob);
```

### export() — render to PixelContainer

```typescript
const pixels = vm.export(scene, { scale: 2 }); // 2× Retina resolution
// pixels: PixelContainer { width, height, channels: 4, bit_depth: 8, pixels: Uint8Array }
const bytes = pngCodec.encode(pixels);          // → Uint8Array of PNG bytes
```

`export()` requires `OffscreenCanvas`. In environments without it
(Node.js without `node-canvas`), it throws `ExportNotSupportedError`.

### Gradients

Gradients are pre-declared as `PaintGradient` instructions, then referenced
by fill with `"url(#id)"`:

```typescript
vm.execute(
  paintScene(400, 200, "transparent", [
    paintGradient("linear",
      [{ offset: 0, color: "#3b82f6" }, { offset: 1, color: "#8b5cf6" }],
      { id: "sky", x1: 0, y1: 0, x2: 400, y2: 0 },
    ),
    paintRect(0, 0, 400, 200, { fill: "url(#sky)" }),
  ]),
  ctx,
);
```

The gradient registry is cleared on each `execute()` call. Gradients do not
persist across scenes.

### Layers — filters and blend modes

```typescript
vm.execute(
  paintScene(400, 300, "#ffffff", [
    paintLayer(
      [paintRect(50, 50, 200, 100, { fill: "#3b82f6" })],
      {
        filters: [
          { kind: "drop_shadow", dx: 4, dy: 4, blur: 8, color: "rgba(0,0,0,0.3)" },
          { kind: "blur", radius: 2 },
        ],
        blend_mode: "multiply",
        opacity: 0.9,
      },
    ),
  ]),
  ctx,
);
```

Supported CSS filter kinds: `blur`, `drop_shadow`, `brightness`, `contrast`,
`saturate`, `hue_rotate`, `invert`, `opacity`. `color_matrix` is skipped
(no CSS filter equivalent — use a WebGL/WASM shader for that).

## API

### `createCanvasVM(): PaintVM<CanvasRenderingContext2D>`

Returns a VM with all 10 instruction handlers registered.

### `resolveFill(fill: string, ctx: CanvasRenderingContext2D): string | CanvasGradient`

Resolves a `"url(#id)"` fill reference to a `CanvasGradient` from the
registry. Pass-through for plain CSS colors.

## Related packages

| Package | Role |
|---|---|
| `@coding-adventures/paint-instructions` | Instruction types and builder helpers |
| `@coding-adventures/paint-vm` | Generic VM engine (dispatch table, patch diff) |
| `@coding-adventures/paint-vm-svg` | SVG string backend |
| `@coding-adventures/paint-vm-canvas` | Canvas 2D backend (this package) |

## Spec

[P2D03 — Paint VM Canvas](../../../specs/P2D03-paint-vm-canvas.md)
