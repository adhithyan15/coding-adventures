# Changelog — caesar-cipher (Java)

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of the Caesar shift cipher.
- `encrypt(text, shift)` — shifts letters forward by `shift` mod 26; negative shifts work; non-alpha unchanged; case preserved.
- `decrypt(text, shift)` — shifts backward; delegates to `encrypt(text, -shift)`.
- `rot13(text)` — Caesar with shift=13; self-inverse.
- `bruteForce(ciphertext)` — returns all 25 non-trivial (shift, plaintext) candidates as `List<BruteForceResult>`.
- `frequencyAnalysis(ciphertext)` — chi-squared attack against English letter frequencies; returns best (shift, plaintext) guess.
- `ENGLISH_FREQUENCIES` — public `double[]` of 26 letter frequencies (A–Z).
- `BruteForceResult` and `FrequencyResult` inner classes.
- Literate source with inline shift table, historical context, and security discussion.
- 26 unit tests covering: shift arithmetic (mod 26, negatives, large values), case preservation, non-alpha passthrough, ROT13 self-inverse, brute force size/content, frequency analysis on long English texts (shift=3 and shift=13).
