# Changelog — sha256 (Kotlin)

## 0.1.0 — 2026-07-11

### Added

- Initial release: pure-Kotlin port of the `sha256` reference package — the
  third language (after Dart and Java) in the pure-port + native campaign.
- `Sha256.sha256(ByteArray)` → 32-byte FIPS 180-4 digest.
- `Sha256.sha256Hex(ByteArray)` → 64-char lowercase hex digest.
- `Sha256.Hasher` — streaming hasher with `update`, non-destructive `digest` /
  `hexDigest`, and `copy` for independent snapshots.
- Uses Kotlin's native 32-bit `Int` arithmetic (`ushr`, `Int.rotateRight`,
  `and`/`or`/`xor`/`inv`) — no masking; constants > 0x7FFFFFFF via `.toInt()`.
- 20 tests (kotlin.test): the four FIPS 180-4 vectors (incl. one-million-'a'),
  output-format/determinism/avalanche properties, padding/block-boundary edge
  cases (55/56/63/64/127/128), and streaming parity with the one-shot API.
