# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## 0.1.0

- Add a pure Haskell coordinator for Code 39, Codabar, Code 128, EAN-13,
  ITF, and UPC-A.
- Preserve typed encoder and layout failures across the shared pipeline.
- Render backend-neutral paint scenes through the pure ASCII Paint VM.
