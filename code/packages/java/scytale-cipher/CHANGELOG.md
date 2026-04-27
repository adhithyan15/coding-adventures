# Changelog — scytale-cipher (Java)

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of the Scytale columnar transposition cipher.
- `encrypt(text, key)` — writes row-by-row into `key` columns with space padding, reads column-by-column.
- `decrypt(text, key)` — inverse transposition; strips trailing padding spaces.
- `bruteForce(text)` — tries keys 2 to `text.length / 2`; returns `List<BruteForceResult>`.
- Input validation: `key < 2` or `key > text.length` throws `IllegalArgumentException`.
- `BruteForceResult` inner class with `key` and `text` fields.
- Literate source with grid diagram, historical context (Spartan scytale), and security discussion.
- 22 unit tests covering: basic encryption/decryption, padding, roundtrip for multiple texts and keys, input validation, brute force.
