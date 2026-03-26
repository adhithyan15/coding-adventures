# @ca/draw-instructions

Backend-neutral 2D draw instructions for reusable scene generation.

This package is intentionally small. It defines a shared scene model that
producer packages can target and renderer packages can consume.

## Primitives

- `DrawScene`
- `DrawRectInstruction`
- `DrawTextInstruction`
- `DrawGroupInstruction`
- `DrawRenderer<Output>`

## Usage

```typescript
import { createScene, drawRect, drawText } from "@ca/draw-instructions";

const scene = createScene(100, 50, [
  drawRect(10, 10, 20, 30, "#000000"),
  drawText(50, 44, "hello"),
]);
```

## Why it exists

- barcode symbologies should not know about SVG
- SVG renderers should not know about barcodes
- future backends can render the same scene to PNG, Canvas, or terminal output
