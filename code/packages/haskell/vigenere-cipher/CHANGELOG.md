# Changelog

## Unreleased - 2026-08-26

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.
- Execute all 26 normative Vigenere fixture objects through the generated native consumer.
- Align Vigenere transforms and cryptanalysis with CR03 ASCII, limit, ordering, 90%-threshold, tie, and exact-key-length behavior.
- Add direct native regressions for Unicode pass-through, preflight limits, and deterministic low-signal analysis.


## 0.1.0

- Add case-preserving Vigenere encryption and decryption.
- Add index-of-coincidence key-length estimation.
- Add chi-squared key recovery and automatic cipher breaking.
