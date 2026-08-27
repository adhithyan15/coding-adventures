# Changelog — atbash-cipher (Kotlin)

## [Unreleased]

### Added

- Execute all six normative `classical-ciphers-v1` Atbash objects in generated,
  dependency-free native tests pinned to the corpus digest and case roster.

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of the Atbash substitution cipher as an idiomatic Kotlin `object`.
- `encrypt(text)` — maps every letter to its mirror in the alphabet via `when` expression; non-alpha unchanged; case preserved.
- `decrypt(text)` — delegates to `encrypt` because Atbash is self-inverse.
- Literate source with inline alphabet mirror table, historical context, and self-inverse proof.
- 24 unit tests covering: individual letters, full alphabet, case preservation, non-alpha pass-through, roundtrip (self-inverse property for all 26 letters), and known cross-language test vectors.
