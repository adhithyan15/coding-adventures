# Changelog

## Unreleased - 2026-08-26

- Add a generated native consumer for all 18 language-neutral Scytale fixture cases.
- Align grid cells with CR02 Unicode scalar values rather than UTF-16 code units.
- Define ragged-ciphertext decryption and remove only trailing U+0020 padding.
- Reject brute-force inputs above 4096 scalars and add a schema-v1 capability profile.
- Refresh the locked development dependency graph so `nanoid` includes its
  audited security fix.

## 0.1.0 — 2026-04-03

- Initial release.
- `encrypt(text, key)` — Scytale transposition encryption.
- `decrypt(text, key)` — Scytale transposition decryption with padding removal.
- `bruteForce(text)` — try all keys from 2 to len/2.
- Full test suite with vitest.
