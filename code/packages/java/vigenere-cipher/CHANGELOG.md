# Changelog — vigenere-cipher (Java)

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of the Vigenère polyalphabetic substitution cipher as
  a static utility class.
- `encrypt(plaintext, key)` — shifts each alphabetic character by the
  corresponding key letter; non-alpha characters pass through unchanged without
  advancing the key position.
- `decrypt(ciphertext, key)` — inverse shift, using `(c - shift + 26) % 26`.
- `findKeyLength(ciphertext, maxLength)` — Index of Coincidence analysis to
  estimate the key length; collects all candidates within 5% of the best
  average IC, removes multiples of smaller candidates to avoid returning `2k`
  instead of `k`, and returns the smallest residual.
- `findKey(ciphertext, keyLength)` — chi-squared frequency analysis per group
  to recover each keyword letter; includes minimal-period detection so that
  even if `findKeyLength` returns a multiple of the true key, the correct
  shorter key is returned.
- `breakCipher(ciphertext)` — fully automatic ciphertext-only attack chaining
  `findKeyLength` → `findKey` → `decrypt`; returns a `BreakResult` record.
- `ENGLISH_FREQUENCIES` — public `double[]` of English letter frequencies
  indexed A=0 … Z=25.
- `BreakResult` — inner static final class holding the recovered `key` and
  `plaintext`.
- Input validation via `validateKey()`: empty key or non-alpha key throws
  `IllegalArgumentException`.
- Literate source with historical context, IC formula, and inline diagrams.
- 22 unit tests covering: encryption/decryption, roundtrip, non-alpha handling,
  key validation, IC key-length detection, chi-squared key recovery, and full
  end-to-end break.
