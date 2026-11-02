# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## 0.1.0

- Validate body-only and explicitly guarded Codabar input.
- Support configurable `A`-`D` start and stop symbols.
- Encode all 20 standard symbols with exact binary patterns.
- Expand attributed start, data, stop, and inter-character-gap runs.
- Emit backend-neutral paint scenes through the shared 1D barcode geometry.
