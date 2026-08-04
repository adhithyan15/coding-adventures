# barcode-1d (Haskell)

Pure Haskell coordinator for the repository's one-dimensional barcode
packages.

The pipeline is:

`symbology package -> barcode-layout-1d -> PaintScene -> paint-vm-ascii`

## Supported symbologies

- Code 39 (the default)
- Codabar, with configurable start and stop guards
- Code 128 Code Set B
- EAN-13
- Interleaved 2 of 5 (ITF)
- UPC-A

Names are case-insensitive and ignore hyphens and underscores, so `ean-13`,
`EAN_13`, and `ean13` select the same encoder. An empty name selects Code 39.

## API

- `normalizeSymbology` validates a user-facing symbology name.
- `buildScene` routes typed options to the selected native Haskell encoder.
- `buildSceneForSymbology` combines string normalization and scene building.
- `renderAscii`, `renderAsciiForSymbology`, and `renderAsciiWithOptions` feed
  the scene through the pure ASCII Paint VM.
- `currentBackend` reports `"ascii"`.

Every failure remains explicit in `Either Barcode1DPipelineError`; the
coordinator does not throw or erase the originating package error.

There is not yet a native Haskell raster Paint VM in this repository, so this
package intentionally does not claim pixel or PNG output. The returned
`PaintScene` remains backend-neutral and can be consumed by future renderers.

## Development

```sh
cabal test all
cabal test all --enable-coverage
cabal check
```
