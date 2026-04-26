# Changelog — scytale-cipher (Kotlin)

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of the Scytale columnar transposition cipher as an idiomatic Kotlin `object`.
- `encrypt(text, key)` — writes row-by-row with `padEnd` space padding, reads column-by-column via `buildString`.
- `decrypt(text, key)` — inverse transposition; uses `trimEnd` to strip padding spaces.
- `bruteForce(text)` — tries keys 2 to `text.length / 2`; returns `List<BruteForceResult>` (data class).
- Input validation via `require()`.
- Literate source with grid diagram, historical context, and security discussion.
- 17 unit tests covering: basic encryption/decryption, padding, roundtrip, input validation, and brute force.
