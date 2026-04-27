# Changelog — vigenere-cipher (Kotlin)

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of the Vigenère polyalphabetic substitution cipher as
  an idiomatic Kotlin `object`.
- `encrypt(text, key)` — shifts each alphabetic character by the corresponding
  key letter; non-alpha characters pass through unchanged without advancing the
  key position.
- `decrypt(text, key)` — inverse shift, using `(c - shift + 26) % 26`.
- `findKeyLength(ciphertext, maxLength)` — Index of Coincidence analysis to
  estimate the key length; collects all candidates within 5% of the best
  average IC and removes multiples of smaller candidates, then returns the
  smallest residual to avoid returning `2k` or `3k` instead of the true `k`.
- `findKey(ciphertext, keyLength)` — chi-squared frequency analysis per group
  to recover each keyword letter; includes minimal-period detection so that
  even if `findKeyLength` returns a multiple of the true key, the correct
  shorter key is returned.
- `breakCipher(ciphertext)` — fully automatic ciphertext-only attack chaining
  `findKeyLength` → `findKey` → `decrypt`; returns a `BreakResult` data class.
- `ENGLISH_FREQUENCIES` — public `DoubleArray` of English letter frequencies
  indexed A=0 … Z=25.
- `BreakResult` — `data class` holding the recovered `key` and `plaintext`.
- Input validation via `require()`: empty key or non-alpha key throws
  `IllegalArgumentException`.
- Literate source with historical context, IC formula, and inline diagrams.
- 22 unit tests covering: encryption/decryption, roundtrip, non-alpha handling,
  key validation, IC key-length detection, chi-squared key recovery, full
  end-to-end break, and `BreakResult` data class behaviour.
