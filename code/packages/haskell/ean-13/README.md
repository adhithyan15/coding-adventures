# ean-13 (Haskell)

Pure EAN-13 encoding for the Haskell package lane. The package accepts a
12-digit payload and computes its required check digit, or accepts a complete
13-digit code and verifies the supplied check digit.

```haskell
import CodingAdventures.Ean13

example = drawEan13 "400638133393" defaultPaintBarcode1DOptions
```

The API exposes all thirty L/G/R digit patterns, every leading-digit parity
sequence, normalized data, typed per-digit encodings, source attribution, and
the complete 95-module run stream. The leading digit is represented indirectly
through the six left-side parity choices, exactly as the symbology specifies.
EAN-2/EAN-5 supplements, registry lookup, and scanner-side decoding remain
outside the V1 contract.

Rendering delegates to `barcode-layout-1d`, which supplies validated quiet-zone
geometry, explicit symbol spans, rectangle-only paint instructions, and shared
scene metadata. Human-readable text remains explicitly unsupported until the
Haskell lane has a text-shaping backend.

## Development

```sh
cabal test all --enable-coverage
```
