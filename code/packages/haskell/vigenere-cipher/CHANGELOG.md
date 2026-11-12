# Changelog

## Unreleased - 2026-08-26

- `BUILD_windows` now runs `cabal test all` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.
- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.
- Execute all 26 normative Vigenere fixture objects through the generated native consumer.
- Align Vigenere transforms and cryptanalysis with CR03 ASCII, limit, ordering, 90%-threshold, tie, and exact-key-length behavior.
- Add direct native regressions for Unicode pass-through, preflight limits, and deterministic low-signal analysis.


## 0.1.0

- Add case-preserving Vigenere encryption and decryption.
- Add index-of-coincidence key-length estimation.
- Add chi-squared key recovery and automatic cipher breaking.
