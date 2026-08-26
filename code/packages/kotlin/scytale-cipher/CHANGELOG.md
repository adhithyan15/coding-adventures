# Changelog — scytale-cipher (Kotlin)

## Unreleased - 2026-08-26

- Align grid cells and key bounds with CR02 Unicode scalar values.
- Define ragged-ciphertext decryption and remove only trailing U+0020 padding.
- Reject brute-force inputs above 4096 scalars and add a pure capability profile.

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of the Scytale columnar transposition cipher as an idiomatic Kotlin `object`.
- `encrypt(text, key)` — writes row-by-row with `padEnd` space padding, reads column-by-column via `buildString`.
- `decrypt(text, key)` — inverse transposition; uses `trimEnd` to strip padding spaces.
- `bruteForce(text)` — tries keys 2 to `text.length / 2`; returns `List<BruteForceResult>` (data class).
- Input validation via `require()`.
- Literate source with grid diagram, historical context, and security discussion.
- 17 unit tests covering: basic encryption/decryption, padding, roundtrip, input validation, and brute force.
