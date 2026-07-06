# Changelog — unicode-normalize

## 0.1.0 — Unreleased

Initial release: zero-dependency Unicode canonical normalization (NFD/NFC) and
combining-mark detection for Unicode 17.0.0, created to remove the third-party
`unicode-normalization` crate from the Engram stack (Engram zero-dependency
program, `code/specs/engram-zero-dep-plan.md`, Phase C).

### Added

- `UnicodeNormalize` trait with `nfd()` / `nfc()` for `&str` and `str::Chars`,
  mirroring the surface Engram consumes.
- `char::is_combining_mark` and `char::canonical_combining_class`.
- `UNICODE_VERSION` = (17, 0, 0).
- Generated canonical tables (CCC, recursive decomposition, composition, Mark
  ranges) driving binary-search lookups; Hangul handled algorithmically.

### Verified

- A throwaway cross-check (removed with the `unicode-normalization` dev-dep)
  asserted a match against the real crate across **every Unicode scalar value**
  (~1.1M code points: CCC, `is_combining_mark`, single-char NFD, single-char NFC)
  **and 200,000 random multi-character strings** (NFD/NFC) — zero mismatches.
