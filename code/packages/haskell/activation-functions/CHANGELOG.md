# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## 0.1.0 - 2026-07-12

- Add pure Haskell implementations of six activation functions and their
  derivatives.
- Preserve the consensus overflow guards and numerically stable softplus
  behavior.
- Add reference-value, derivative, sweep, and extreme-input tests.
