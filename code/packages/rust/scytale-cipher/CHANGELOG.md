# Changelog

## Unreleased - 2026-08-26

- Add a generated native consumer for all 18 language-neutral Scytale fixture cases.
- Pin CR02 Unicode-scalar, combining-sequence, ragged-grid, and literal-padding behavior.
- Accept signed keys so empty input still returns before key validation.
- Return checked brute-force results and reject inputs above 4096 scalars before allocation.

## 0.1.0 — 2026-04-03

- Initial release.
- `encrypt(text, key)` — Scytale transposition encryption.
- `decrypt(text, key)` — Scytale transposition decryption with padding removal.
- `brute_force(text)` — try all keys from 2 to len/2.
- Full test suite.
