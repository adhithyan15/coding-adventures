# Changelog

## Unreleased - 2026-08-26

- Pin CR02 Unicode-scalar, combining-sequence, ragged-grid, and literal-padding behavior.
- Materialize scalar arrays for portable indexing and remove only trailing U+0020 padding.
- Reject brute-force inputs above 4096 scalars and add a schema-v1 capability profile.

## 0.1.0 — 2026-04-03

- Initial release.
- `encrypt(text, key)` — Scytale transposition encryption.
- `decrypt(text, key)` — Scytale transposition decryption with padding removal.
- `brute_force(text)` — try all keys from 2 to len/2.
- Full test suite with minitest.
