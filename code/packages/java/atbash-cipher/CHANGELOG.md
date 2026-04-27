# Changelog — atbash-cipher (Java)

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of the Atbash substitution cipher.
- `encrypt(text)` — maps every letter to its mirror in the alphabet; non-alpha unchanged; case preserved.
- `decrypt(text)` — identical to `encrypt` because Atbash is self-inverse.
- Literate source with inline alphabet mirror table, historical context, and self-inverse proof.
- 18 unit tests covering: individual letters, full alphabet, case preservation, non-alpha pass-through, roundtrip (self-inverse property for all 26 letters), and known cross-language test vectors.
