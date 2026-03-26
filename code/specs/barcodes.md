# Barcodes

## Overview

This spec defines the shared architecture for barcode work in the
coding-adventures monorepo.

The barcode track will be built incrementally, one symbology at a time. Each
format gets its own spec so that implementation, tests, learning material, and
visualization can grow independently.

The first target is **Code 39** because it is simple, teachable, and works well
for a dependency-free first implementation.

## Goals

- build barcode encoders from first principles
- avoid external barcode libraries by default
- expose intermediate structures for teaching and visualization
- support multiple barcode formats over time without collapsing them into one
  vague API

## Spec Layout

Shared barcode architecture lives here. Format-specific behavior lives in
separate spec files:

- [draw-instructions.md](/Users/adhithya/Downloads/Codex/coding-adventures/code/specs/draw-instructions.md)
- [draw-instructions-svg.md](/Users/adhithya/Downloads/Codex/coding-adventures/code/specs/draw-instructions-svg.md)
- [code39.md](/Users/adhithya/Downloads/Codex/coding-adventures/code/specs/code39.md)
- [upc-a.md](/Users/adhithya/Downloads/Codex/coding-adventures/code/specs/upc-a.md)
- [ean-13.md](/Users/adhithya/Downloads/Codex/coding-adventures/code/specs/ean-13.md)

Additional symbologies should follow the same pattern.

## Shared Design Principles

Every barcode implementation in this repo should separate:

1. input validation and normalization
2. symbology-specific encoding
3. expansion into machine-readable bar/space structure
4. translation into backend-neutral draw instructions
5. rendering into a native output format

The learning and visualization story depends on keeping those layers explicit.
A final image by itself is not enough.

## Common API Shape

At a high level, barcode implementations in this repo should follow:

```text
input data
  -> normalize and validate
  -> encode according to a specific symbology
  -> expand into bars/spaces or modules
  -> translate into draw instructions
  -> render to a native output format
```

For 1D barcodes, the first preferred output format is **SVG** because it is:

- dependency-free
- inspectable in tests
- easy to generate with string building alone
- a natural fit for browser-based visualizers

## Initial Implementation Decision

The first implementation should treat:

- **input** as the data to encode
- **output** as a backend-rendered result

The first built-in backend should return an SVG string, but the same draw
instructions should later be renderable to PNG, Canvas, terminal output, or
other frontends.

The package should still expose intermediate structures for the visualizer:

- encoded symbols
- run/module sequence
- draw-scene metadata

## Future Extensions

- Code 39 checksum support
- UPC-A
- EAN-13
- Code 128
- QR codes
- barcode reader/scanner simulation
- shared JSON explanations for visualizer apps
