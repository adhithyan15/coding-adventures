# itf (Haskell)

Pure Interleaved 2 of 5 (ITF) barcode encoding for the Haskell package lane.
The first digit in each pair controls bar widths, while the second controls
space widths. Inputs must contain a non-empty, even number of ASCII digits.

```haskell
import CodingAdventures.Itf

example = drawItf "123456" defaultPaintBarcode1DOptions
```

The encoder exposes normalized payloads, typed pair records, and the expanded
run stream. Rendering delegates to `barcode-layout-1d`, so callers receive the
same validated quiet-zone geometry, explicit symbol spans, rectangle-only
paint instructions, and metadata as the other linear symbologies.

V1 deliberately excludes ITF-14 bearer bars, GTIN packaging semantics, and
automatic check-digit insertion.

## Development

```sh
cabal test all --enable-coverage
```
