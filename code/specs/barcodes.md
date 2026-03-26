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
- [barcode-1d.md](/Users/adhithya/Downloads/Codex/coding-adventures/code/specs/barcode-1d.md)
- [code39.md](/Users/adhithya/Downloads/Codex/coding-adventures/code/specs/code39.md)
- [upc-a.md](/Users/adhithya/Downloads/Codex/coding-adventures/code/specs/upc-a.md)
- [ean-13.md](/Users/adhithya/Downloads/Codex/coding-adventures/code/specs/ean-13.md)
- [codabar.md](/Users/adhithya/Downloads/Codex/coding-adventures/code/specs/codabar.md)
- [itf.md](/Users/adhithya/Downloads/Codex/coding-adventures/code/specs/itf.md)
- [code128.md](/Users/adhithya/Downloads/Codex/coding-adventures/code/specs/code128.md)

Additional symbologies should follow the same pattern.

## Shared Design Principles

Every barcode implementation in this repo should separate:

1. input validation and normalization
2. symbology-specific encoding
3. expansion into machine-readable bar/space or module structure
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

## Shared 1D Decision

All linear symbologies should target a shared 1D barcode abstraction before
they target draw instructions.

That means the pipeline for UPC-A, EAN-13, Codabar, ITF, Code 39, and Code 128
should look like:

```text
input
  -> normalize and validate
  -> encode symbology symbols
  -> expand into linear runs with numeric module widths
  -> translate runs into draw instructions
  -> render with SVG or another backend
```

This matters because wide/narrow formats and module-based retail formats are
different at the symbology level, but they become the same kind of geometry:
a left-to-right stream of bars and spaces with widths measured in modules.

The shared 1D package should own:

- a generic run model with numeric widths
- quiet-zone handling
- default bar rendering into draw instructions
- optional human-readable labels
- layout metadata that visualizers can inspect

Format-specific packages should own:

- allowed character set
- checksum rules
- start/stop and guard patterns
- parity or code-set rules
- symbol tables

## Future Extensions

- Code 39 checksum support
- UPC-A
- EAN-13
- Code 128
- QR codes
- barcode reader/scanner simulation
- shared JSON explanations for visualizer apps
