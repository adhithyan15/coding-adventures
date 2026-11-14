# Changelog — caesar-cipher (Kotlin)

## [Unreleased]

### Fixed

- SEC-010: `decrypt` negated the raw shift (Kotlin `Int`). `decrypt` now reduces the shift to 0..25 before negating it. `-Int.MIN_VALUE` wraps back to `Int.MIN_VALUE`, so the undo shifted by +2 instead of -2 and returned wrong plaintext. Tests: `decrypt` at the minimum shift against a by-hand value, plus `decrypt(encrypt(t, s), s) == t` for s in {min, min+1, -1, 0, 25, 26, max}. `encrypt` was already safe (`shiftChar` reduces first).

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of the Caesar shift cipher as an idiomatic Kotlin `object`.
- `encrypt(text, shift)` — shifts letters forward by `shift` mod 26; negative shifts work; non-alpha unchanged; case preserved.
- `decrypt(text, shift)` — delegates to `encrypt(text, -shift)`.
- `rot13(text)` — Caesar with shift=13; self-inverse.
- `bruteForce(ciphertext)` — returns all 25 non-trivial candidates as `List<BruteForceResult>` (data class).
- `frequencyAnalysis(ciphertext)` — chi-squared attack against English letter frequencies; returns `FrequencyResult` (data class).
- `ENGLISH_FREQUENCIES` — public `DoubleArray` of 26 letter frequencies (A–Z).
- Literate source with inline shift table, historical context, and security discussion.
- 26 unit tests covering: shift arithmetic (mod 26, negatives, large values), case preservation, non-alpha passthrough, ROT13 self-inverse, brute force size/content, frequency analysis on long English texts (shift=3 and shift=13).
