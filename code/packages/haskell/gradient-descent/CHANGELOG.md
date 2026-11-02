# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## 0.1.0 - 2026-07-19

- Add a pure stochastic-gradient-descent update matching the ML02 contract.
- Reject empty and length-mismatched weight and gradient vectors explicitly.
- Cover reference values, learning-rate behavior, input preservation, and all
  validation paths with package-native tests.
