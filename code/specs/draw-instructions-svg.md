# Draw Instructions SVG Renderer

## Overview

This spec defines an SVG renderer for the backend-neutral draw instructions
package.

It consumes a generic `DrawScene` and produces an SVG string. It knows how to
serialize draw instructions, but it does not know anything about barcodes or
other producer domains.

## Goals

- take generic draw instructions and emit valid SVG
- keep the renderer dependency-free
- preserve scene metadata where helpful
- be reusable for both 1D and 2D barcode formats

## Supported Instructions

V1 should render:

- `rect`
- `text`
- `group`

## Public API Shape

```typescript
function renderSvg(scene: DrawScene): string
```

## Renderer Rules

- output a complete `<svg>` document string
- include `width`, `height`, and `viewBox`
- render a background rectangle from scene background
- map draw rectangles to `<rect>`
- map draw text to `<text>`
- map groups recursively to `<g>`
- escape text and attribute values safely

## Future Extensions

- style helpers
- defs support
- gradients and patterns
- path rendering
- accessibility helpers
