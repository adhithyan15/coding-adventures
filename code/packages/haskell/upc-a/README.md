# upc-a (Haskell)

Pure UPC-A encoding for the Haskell package lane. The package accepts an
11-digit payload and computes its required check digit, or accepts a complete
12-digit code and verifies the supplied check digit.

```haskell
import CodingAdventures.UpcA

example = drawUpcA "03600029145" defaultPaintBarcode1DOptions
```

The API exposes typed left/right encodings, all twenty standard digit patterns,
the modulo-10 checksum, normalized data, per-digit source attribution, and the
complete 95-module run stream. UPC-E compression, supplements, GS1 application
identifiers, and scanner-side decoding remain outside the V1 contract.

Rendering delegates to `barcode-layout-1d`, which supplies validated quiet-zone
geometry, explicit symbol spans, rectangle-only paint instructions, and shared
scene metadata. Human-readable text remains explicitly unsupported until the
Haskell lane has a text-shaping backend.

## Development

```sh
cabal test all --enable-coverage
```
