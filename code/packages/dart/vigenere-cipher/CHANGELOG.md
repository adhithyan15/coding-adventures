# Changelog

## Unreleased - 2026-08-26

- Execute all 26 normative Vigenere fixture objects through the generated native consumer.
- Align Vigenere transforms and cryptanalysis with CR03 ASCII, limit, ordering, 90%-threshold, tie, and exact-key-length behavior.
- Add direct native regressions for Unicode pass-through, preflight limits, and deterministic low-signal analysis.


## 0.1.0 - 2026-08-26

### Added

- Add CR03 ASCII Vigenère encryption and decryption.
- Add bounded IC key-length estimation, chi-squared key recovery, and complete
  cipher breaking with deterministic short-input behavior.
- Cover parity vectors, Unicode pass-through, hostile keys, analysis limits,
  and long-English key recovery.
- Add strict cross-platform build gates and an empty authority profile.
