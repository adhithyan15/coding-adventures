# Changelog — coding_adventures_sha256

## 0.1.0 — 2026-07-11

### Added

- Initial release: pure-Dart port of the `sha256` reference package.
- `sha256(List<int>)` → `Uint8List` — 32-byte FIPS 180-4 digest.
- `sha256Hex(List<int>)` → `String` — 64-char lowercase hex digest.
- `Sha256Hasher` — streaming hasher with `update`, non-destructive `digest` /
  `hexDigest`, and `cloneHasher` for independent snapshots.
- Correct 32-bit wrap-around arithmetic on Dart's 64-bit `int` via `& 0xFFFFFFFF`
  masking and logical (`>>>`) shifts.
- 23 unit tests: the four FIPS 180-4 vectors (incl. the one-million-'a' case),
  output-format and determinism/avalanche properties, padding/block-boundary
  edge cases (55/56/63/64/127/128), and streaming parity with the one-shot API
  (byte-at-a-time, block-split, clone independence, one-million-'a' streamed).
