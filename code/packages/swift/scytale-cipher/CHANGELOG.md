# Changelog

## Unreleased - 2026-08-26

- Align grid cells with CR02 Unicode scalar values rather than grapheme clusters.
- Define ragged-ciphertext decryption and remove only trailing U+0020 padding.
- Make brute force throwing and reject inputs above 4096 scalars before allocation.

## 0.1.0 — 2026-04-03

- Initial release.
- `encrypt(_:key:)` — Scytale transposition encryption.
- `decrypt(_:key:)` — Scytale transposition decryption with padding removal.
- `bruteForce(_:)` — try all keys from 2 to len/2.
- Full XCTest test suite.
