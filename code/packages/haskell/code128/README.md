# code128 (Haskell)

Pure Code 128 encoding for the Haskell package lane. V1 implements Code Set B
for printable ASCII, including the required Start B symbol, weighted modulo-103
checksum, and stop pattern.

```haskell
import CodingAdventures.Code128

example = drawCode128 "Code 128" defaultPaintBarcode1DOptions
```

The API exposes validated input, character values, all 107 standard patterns,
typed per-symbol encodings, and the complete attributed run stream. Automatic
Code Set A/C switching, FNC behavior, GS1-128, and Code Set C compaction remain
outside the V1 contract.

Rendering delegates to `barcode-layout-1d`, which supplies validated quiet-zone
geometry, rectangle-only paint instructions, and shared scene metadata. Human-
readable text remains explicitly unsupported until the Haskell lane has a text
shaping backend.

## Development

```sh
cabal test all --enable-coverage
```
