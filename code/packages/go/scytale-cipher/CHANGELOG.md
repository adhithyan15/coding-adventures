# Changelog

## Unreleased - 2026-08-26

- Add a generated native consumer for all 18 language-neutral Scytale fixture cases.
- Pin CR02 Unicode-scalar, combining-sequence, ragged-grid, and literal-padding behavior.
- Return a checked brute-force result and reject inputs above 4096 scalars before allocation.
- Add an explicit pure-computation capability profile.

## 0.1.0 — 2026-04-03

- Initial release.
- `Encrypt(text, key)` — Scytale transposition encryption.
- `Decrypt(text, key)` — Scytale transposition decryption with padding removal.
- `BruteForce(text)` — try all keys from 2 to len/2.
- Full test suite with comprehensive coverage.
