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

### Rendering infrastructure

- [draw-instructions.md](draw-instructions.md)
- [draw-instructions-svg.md](draw-instructions-svg.md)
- [draw-instructions-metal.md](draw-instructions-metal.md)

### 1D (linear) barcodes

- [barcode-1d.md](barcode-1d.md) — shared 1D abstraction (runs model)
- [code39.md](code39.md) — Code 39
- [upc-a.md](upc-a.md) — UPC-A
- [ean-13.md](ean-13.md) — EAN-13
- [codabar.md](codabar.md) — Codabar
- [itf.md](itf.md) — ITF (Interleaved 2 of 5)
- [code128.md](code128.md) — Code 128

### 2D (matrix and stacked) barcodes

- [barcode-2d.md](barcode-2d.md) — shared 2D abstraction (ModuleGrid model)
- [qr-code.md](qr-code.md) — QR Code (v1–40, ECC levels L/M/Q/H)
- data-matrix.md — Data Matrix ECC200 *(planned)*
- aztec-code.md — Aztec Code *(planned)*
- pdf417.md — PDF417 *(planned)*
- micro-qr.md — MicroQR *(planned)*

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
- Mixed-mode QR encoding (numeric/alphanumeric/byte segments)
- QR ECI mode (explicit UTF-8 signal)
- QR Structured Append (split message across symbols)
- MicroQR / rMQR for compact spaces
- Data Matrix ECC200 (industrial marking)
- Aztec Code (boarding passes)
- PDF417 (driver's licences, USPS)
- barcode reader/scanner simulation
- shared JSON explanations for visualizer apps
