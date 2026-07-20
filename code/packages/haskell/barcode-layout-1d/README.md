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

This package is the shared geometry foundation for the remaining
Haskell Code 39, Codabar, ITF, UPC-A, EAN-13, and Code 128 ports.

## Development

```sh
cabal test all
```
