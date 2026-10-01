# barcode-layout-1d (Haskell)

Pure shared geometry for linear barcode symbologies. Encoders provide an
alternating stream of bar and space runs; this package validates those runs,
computes symbol spans and quiet zones, and emits rectangle-only `PaintScene`
values through the existing Haskell `paint-instructions` package.

```haskell
import CodingAdventures.BarcodeLayout1D

example = do
  runs <- runsFromBinaryPattern "101"
    (defaultBinaryPatternOptions "start" (-1) Guard)
  layoutBarcode1D runs defaultPaintBarcode1DOptions
```

The shared defaults use four scene units per module, 120-unit bars, and
ten-module quiet zones on both sides. Run metadata is preserved on each bar
rectangle, and the scene records its content and total module widths.

Human-readable text is rejected explicitly until the Haskell paint stack has
portable text metrics and glyph shaping. This prevents callers from receiving
an incomplete scene that silently omits requested text.

## Portable v1 adapter

`runsFromBinaryPatternV1`, `runsFromWidthPatternV1`,
`computeBarcode1DLayoutV1`, and `projectBarcode1DSceneV1` implement the shared
`barcode-layout-1d-v1` fixture contract. They use bounded integer arithmetic,
the closed payload-blind `Barcode1DV1Error` type with stable IDs via
`barcode1DV1ErrorId`, scalar-valid attribution, canonical string metadata, and
rectangle-only projection. The older entry points retain their payload-bearing
`Barcode1DError` API for source compatibility with established symbology packages.

The native conformance suite dynamically executes all 56 shared cases. Its
two text-precedence checks prove that both text request forms fail before any
native resolution path; the production package has an empty capability
manifest and no font or shaping dependency.

This package is the shared geometry foundation for the remaining
Haskell Code 39, Codabar, ITF, UPC-A, EAN-13, and Code 128 ports.

## Development

```sh
cabal test all
```
