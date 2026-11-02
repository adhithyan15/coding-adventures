# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## 0.1.0 - 2026-07-18

- Add mean squared, mean absolute, binary cross-entropy, and categorical
  cross-entropy losses.
- Add derivatives for every loss and clamp probability inputs away from zero
  and one.
- Reject empty and length-mismatched input vectors explicitly.
