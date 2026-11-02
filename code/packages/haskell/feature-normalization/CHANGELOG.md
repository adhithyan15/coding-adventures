# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## 0.1.0 - 2026-07-18

- Add standard and min-max scaler fitting and transformation.
- Map constant feature columns to zero without dividing by zero.
- Add explicit validation for empty, zero-width, ragged, and mismatched input.
