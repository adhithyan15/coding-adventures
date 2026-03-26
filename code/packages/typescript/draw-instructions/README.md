# @ca/draw-instructions

Backend-neutral 2D draw instructions for reusable scene generation.

This package is intentionally small. It defines a shared scene model that
producer packages can target and renderer packages can consume.

It exists to break a hard coupling that otherwise shows up quickly in learning
projects:

- barcode packages should not need to know SVG syntax
- SVG packages should not need to know barcode symbologies
- future backends should not force us to rewrite producer logic

## Mental Model

Think of this package as the "IR" layer for drawing.

The same way a compiler might separate:

```text
source code -> AST -> machine code
```

we separate visualization into:

```text
domain logic -> draw scene -> backend output
```

For barcodes:

```text
Code 39 encoding -> DrawScene -> SVG string
```

## Primitives

- `DrawScene`
- `DrawRectInstruction`
- `DrawTextInstruction`
- `DrawGroupInstruction`
- `DrawRenderer<Output>`

These primitives are intentionally enough for a lot of work:

- 1D barcode bars are rectangles
- 2D barcode modules are rectangles
- labels are text
- semantic layers can be groups

## Usage

```typescript
import { createScene, drawRect, drawText } from "@ca/draw-instructions";

const scene = createScene(100, 50, [
  drawRect(10, 10, 20, 30, "#000000"),
  drawText(50, 44, "hello"),
]);
```

## Why Rectangles Matter

It may be tempting to create a special primitive such as `Bar` for barcodes,
but that would be the wrong abstraction level for the shared package.

A 1D barcode bar is just:

- a rectangle with small width
- and large height

A 2D barcode module is just:

- a rectangle with small width
- and small height

So `rect` is the right reusable primitive.

## Metadata

Each instruction can optionally carry metadata.

This is important for teaching tools. It lets a producer preserve semantic
meaning without baking that meaning into the shared drawing schema.

Examples:

- a Code 39 package can tag a bar with its source character
- a graph visualizer can tag a node with its node id
- a pipeline visualizer can tag a box with its stage name

## Why it exists

- barcode symbologies should not know about SVG
- SVG renderers should not know about barcodes
- future backends can render the same scene to PNG, Canvas, or terminal output

## Design Constraints

- no backend-specific fields in the scene model
- no barcode-specific concepts in the shared package
- explicit width and height on the scene so renderers do not guess bounds
- small enough API surface that reading the source teaches the whole model
