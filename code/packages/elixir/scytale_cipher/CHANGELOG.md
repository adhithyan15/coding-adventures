# Changelog

## Unreleased - 2026-08-26

- Align grid cells with CR02 Unicode scalar values, including decomposed combining sequences.
- Use tuple-backed scalar grids for bounded indexing and ragged-ciphertext decryption.
- Remove only trailing U+0020 padding and reject brute-force inputs above 4096 scalars.

## 0.1.0 — 2026-04-03

- Initial release.
- `encrypt/2` — Scytale transposition encryption.
- `decrypt/2` — Scytale transposition decryption with padding removal.
- `brute_force/1` — try all keys from 2 to len/2.
- Full ExUnit test suite.
