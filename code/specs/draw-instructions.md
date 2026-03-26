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

## Initial Primitives

V1 should support:

- `Rect`
- `Text`
- `Group`
- `Scene`

These are enough for:

- 1D barcodes: vertical bar rectangles + optional labels
- 2D barcodes: module rectangles in a grid
- future overlays, guides, and explanations

## Public API Shape

```typescript
type DrawInstruction = DrawRect | DrawText | DrawGroup;

interface DrawRect {
  kind: "rect";
  x: number;
  y: number;
  width: number;
  height: number;
  fill: string;
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
  metadata?: Record<string, string | number | boolean>;
}

interface DrawGroup {
  kind: "group";
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

## Design Notes

- Coordinates are 2D even for 1D barcode scenes.
- The scene model should not know anything about SVG.
- The scene model should not know anything about Code 39.
- Metadata is included so producers can preserve semantic information for
  visualizers and interactive frontends.

## Future Extensions

- lines and polylines
- circles and paths
- transforms
- clipping
- z-index / layering helpers
- rasterization backends
