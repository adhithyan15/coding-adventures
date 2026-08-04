# Changelog — coding_adventures_caesar_cipher

## 0.1.0 — 2026-07-10

### Added

- Initial release: pure-Dart port of the `caesar-cipher` reference package.
- `encrypt` / `decrypt` — Caesar shift and its inverse, with shift
  normalisation (negative and out-of-range shifts wrap into 0..25). Case is
  preserved; non-alphabetic and non-ASCII characters pass through unchanged.
- `rot13` — the shift-13 special case, its own inverse.
- `bruteForce` — returns all 25 non-trivial candidate decryptions as
  `BruteForceResult(shift, plaintext)` values.
- `frequencyAnalysis` — chi-squared attack that recovers the most likely shift,
  returning a `({int shift, String plaintext})` record; falls back to shift 1
  when the ciphertext has no letters.
- `englishFrequencies` — the 26-entry English letter-frequency table.
- 21 unit tests covering shifting, case preservation, wrap-around, negative and
  large shifts, non-ASCII passthrough, round-trip over shifts −30..30, ROT13
  self-inverse, brute-force ordering/equality, and frequency-analysis recovery.
