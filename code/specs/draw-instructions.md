# Draw Instructions

## Overview

This spec defines a backend-neutral drawing package for the
coding-adventures monorepo.

The package exists to separate:

- domain logic that decides what should be drawn
- backend logic that decides how to serialize or paint it

For barcode work, this is the shared layer between:

- a symbology package such as Code 39
- a renderer package such as SVG

## Goals

- represent simple 2D scenes with a small set of primitives
- make those primitives reusable across 1D and 2D barcode formats
- avoid coupling encoding logic to SVG, Canvas, PNG, or terminal backends
- keep the model simple enough to inspect in tests

## Primitives

The scene model has five instruction types plus a scene container:

- `Rect` — filled and/or stroked rectangle
- `Text` — positioned text label with font control
- `Group` — hierarchical grouping of children
- `Line` — straight line segment (V2)
- `Clip` — rectangular clipping region for children (V2)
- `Scene` — top-level container with dimensions and background

These cover:

- 1D barcodes: vertical bar rectangles + optional labels
- 2D barcodes: module rectangles in a grid
- Tables: header/row backgrounds (rect), cell text (text + clip), grid lines (line)
- Interactive components: focus rings (stroked rect), selection highlights
- Future overlays, guides, and explanations

## Public API Shape

```typescript
type DrawInstruction = DrawRect | DrawText | DrawGroup | DrawLine | DrawClip;

interface DrawRect {
  kind: "rect";
  x: number;
  y: number;
  width: number;
  height: number;
  fill: string;
  stroke?: string;        // optional border color
  strokeWidth?: number;   // optional border thickness
  metadata?: Record<string, string | number | boolean>;
}

interface DrawText {
  kind: "text";
  x: number;
  y: number;
  value: string;
  fill: string;
  fontFamily: string;
  fontSize: number;
  align: "start" | "middle" | "end";
  fontWeight?: "normal" | "bold";  // optional weight
  metadata?: Record<string, string | number | boolean>;
}

interface DrawGroup {
  kind: "group";
  children: DrawInstruction[];
  metadata?: Record<string, string | number | boolean>;
}

interface DrawLine {
  kind: "line";
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  stroke: string;
  strokeWidth: number;
  metadata?: Record<string, string | number | boolean>;
}

interface DrawClip {
  kind: "clip";
  x: number;
  y: number;
  width: number;
  height: number;
  children: DrawInstruction[];
  metadata?: Record<string, string | number | boolean>;
}

interface DrawScene {
  width: number;
  height: number;
  background: string;
  instructions: DrawInstruction[];
  metadata?: Record<string, string | number | boolean>;
}
```

### Why these primitives are enough

Every 2D rendering backend (SVG, Canvas, Direct2D, Metal, Vulkan, GDI,
terminal text) can implement these five operations. The draw-instructions
model is the universal intermediate representation — producers emit it,
and any backend that implements the renderer interface can consume it.

```
DrawRect    → SVG <rect>         → ctx.fillRect()      → FillRectangle()
DrawText    → SVG <text>         → ctx.fillText()       → DrawText()
DrawLine    → SVG <line>         → ctx.moveTo/lineTo()  → DrawLine()
DrawClip    → SVG <clipPath>     → ctx.clip()           → PushClip()
DrawGroup   → SVG <g>            → (just recurse)       → (just recurse)
```

## Design Notes

- Coordinates are 2D even for 1D barcode scenes.
- The scene model should not know anything about SVG.
- The scene model should not know anything about Code 39.
- Metadata is included so producers can preserve semantic information for
  visualizers and interactive frontends.

## Future Extensions

- circles and ellipses
- paths (arbitrary bezier curves)
- transforms (translate, rotate, scale)
- z-index / layering helpers
- rasterization backends (PNG, PDF)
- Canvas renderer (DrawRenderer<void> that paints to a CanvasRenderingContext2D)
- ASCII/text renderer (DrawRenderer<string> that outputs box-drawing characters)
- native renderers (Direct2D, Metal, Vulkan via FFI from Rust core)
