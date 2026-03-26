# @ca/draw-instructions-svg

SVG renderer for backend-neutral draw instructions.

This package consumes `@ca/draw-instructions` scenes and returns SVG strings.

It is deliberately a renderer only. It should not know what a barcode is, what
an AST node is, or what any producer-specific shape "means". It only knows how
to turn generic rectangles, text, and groups into SVG markup.

## Responsibility Boundary

This package answers one question:

```text
Given a DrawScene, how do we serialize it as SVG?
```

It does not answer:

- what should be in the scene
- how barcode widths are computed
- where labels should go
- how graph layouts are arranged

## Usage

```typescript
import { createScene, drawRect } from "@ca/draw-instructions";
import { renderSvg } from "@ca/draw-instructions-svg";

const scene = createScene(100, 50, [drawRect(10, 10, 20, 30)]);
const svg = renderSvg(scene);
```

## Rendering Rules

V1 keeps the mapping intentionally direct:

- `DrawScene` -> `<svg>`
- scene background -> one background `<rect>`
- `DrawRectInstruction` -> `<rect>`
- `DrawTextInstruction` -> `<text>`
- `DrawGroupInstruction` -> `<g>`

Metadata is emitted as `data-*` attributes so semantic information survives into
the SVG output for debugging and interactive tooling.

## Why SVG First?

SVG is a strong first backend because it is:

- text-based
- easy to inspect in tests
- easy to embed in browser-based visualizers
- straightforward to generate without native image dependencies

Later, the same `DrawScene` can be consumed by a PNG or Canvas backend.
