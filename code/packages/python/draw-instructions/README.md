# draw-instructions

Backend-neutral 2D draw instructions for reusable scene generation.

## Why This Package Exists

This package is the seam between producer logic and renderer logic.

Producer examples:
- Code 39 decides where bars and labels belong
- a graph package decides where nodes and edges belong

Renderer examples:
- SVG
- PNG
- Canvas
- terminal output

That separation keeps the architecture clean. Barcode code should not know SVG
syntax, and SVG code should not know barcode rules.

## Mental Model

```text
domain logic -> DrawScene -> backend output
```

## Primitives

- `DrawRectInstruction`
- `DrawTextInstruction`
- `DrawGroupInstruction`
- `DrawScene`

A 1D barcode bar is just a rectangle with very small width and large height.
A 2D barcode module is also a rectangle. That is why rectangles are the right
shared primitive.

## Development

```bash
bash BUILD
```
