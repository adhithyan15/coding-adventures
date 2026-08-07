# code39 (Haskell)

Pure Code 39 barcode encoding for the Haskell package lane. Lowercase letters
are normalized to uppercase, spaces are preserved, and input is restricted to
the standard alphanumeric and punctuation alphabet. The `*` symbol is reserved
for the start and stop markers inserted by the encoder.

```haskell
import CodingAdventures.Code39

example = drawCode39 "hello-123" defaultPaintBarcode1DOptions
```

The package exposes normalized data, typed per-character encodings, and the
complete attributed run stream. Rendering delegates to `barcode-layout-1d`,
which supplies validated quiet-zone geometry, rectangle-only paint
instructions, and shared scene metadata.

V1 deliberately excludes extended ASCII mode and the optional modulo-43
checksum.

## Development

```sh
cabal test all --enable-coverage
```
