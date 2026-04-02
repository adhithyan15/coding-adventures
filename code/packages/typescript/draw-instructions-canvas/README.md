# @ca/draw-instructions-canvas

Canvas renderer for backend-neutral draw instructions.

This package consumes `@ca/draw-instructions` scenes and paints them directly
to a `CanvasRenderingContext2D`. It skips the DOM entirely — no HTML parsing,
no SVG, no retained-mode element tree.

## Responsibility Boundary

This package answers one question:

```text
Given a DrawScene and a CanvasRenderingContext2D, how do we paint it?
```

It does not answer:

- what should be in the scene
- how a barcode is laid out
- where the canvas comes from (browser, OffscreenCanvas, node-canvas, etc.)

## Usage (browser)

```typescript
import { createScene, drawRect, drawText } from "@ca/draw-instructions";
import { renderCanvas } from "@ca/draw-instructions-canvas";

const canvas = document.getElementById("my-canvas") as HTMLCanvasElement;
const ctx = canvas.getContext("2d")!;

const scene = createScene(400, 200, [
  drawRect(10, 10, 100, 50, "#0070f3"),
  drawText(60, 42, "Hello", { fill: "#ffffff", align: "middle" }),
]);

renderCanvas(scene, ctx);
```

## Usage (OffscreenCanvas, web worker)

```typescript
import { renderCanvas } from "@ca/draw-instructions-canvas";

const offscreen = new OffscreenCanvas(800, 400);
const ctx = offscreen.getContext("2d")!;
renderCanvas(scene, ctx);

const blob = await offscreen.convertToBlob({ type: "image/png" });
```

## Usage (server-side, node-canvas)

```typescript
import { createCanvas } from "canvas"; // npm install canvas
import { renderCanvas } from "@ca/draw-instructions-canvas";

const canvas = createCanvas(800, 400);
renderCanvas(scene, canvas.getContext("2d"));
const png = canvas.toBuffer("image/png");
```

## DrawRenderer factory

When you need to pass the renderer as a value (e.g., to `renderWith()`):

```typescript
import { createCanvasRenderer } from "@ca/draw-instructions-canvas";
import { renderWith } from "@ca/draw-instructions";

const renderer = createCanvasRenderer(ctx);
renderWith(scene, renderer); // same as renderCanvas(scene, ctx)
```

## Rendering Rules

Each draw instruction maps 1-to-1 to Canvas 2D API calls:

| Instruction           | Canvas calls                                         |
|-----------------------|------------------------------------------------------|
| `DrawScene` background| `fillStyle = bg; fillRect(0, 0, w, h)`              |
| `DrawRectInstruction` | `fillRect()` + optional `strokeRect()`             |
| `DrawTextInstruction` | `font = …; fillText()`                             |
| `DrawLineInstruction` | `beginPath(); moveTo(); lineTo(); stroke()`         |
| `DrawGroupInstruction`| recurse children (no state change)                  |
| `DrawClipInstruction` | `save(); beginPath(); rect(); clip(); … restore()` |

### Alignment

`DrawTextInstruction.align` uses `"start" | "middle" | "end"`.
Canvas's `textAlign` uses `"start" | "center" | "end"`.

The renderer maps `"middle"` → `"center"` automatically so text is centered
correctly without any manual conversion by the caller.

## Why Canvas vs SVG?

| | SVG | Canvas |
|---|---|---|
| Output | Text (DOM) | Pixels (raster) |
| Retained mode | Yes — browser tracks element tree | No — immediate mode |
| Per-element interaction | Easy (`addEventListener`) | Manual hit-testing |
| Large scenes | Slower (DOM overhead) | Faster (direct rasterize) |
| Off-screen rendering | Limited | OffscreenCanvas + workers |
| Server-side | svgdom / cheerio needed | node-canvas available |

Use SVG when you need selectable text, accessibility trees, or interactive
elements. Use Canvas when you need raw rendering throughput or off-screen
compositing.
