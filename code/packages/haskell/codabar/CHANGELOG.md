# Changelog

## Unreleased

- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## 0.1.0

- Validate body-only and explicitly guarded Codabar input.
- Support configurable `A`-`D` start and stop symbols.
- Encode all 20 standard symbols with exact binary patterns.
- Expand attributed start, data, stop, and inter-character-gap runs.
- Emit backend-neutral paint scenes through the shared 1D barcode geometry.
