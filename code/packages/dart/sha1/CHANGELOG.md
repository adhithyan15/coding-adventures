# Changelog — coding_adventures_sha1

## 0.1.0 — 2026-07-11

### Added

- Initial release: pure-Dart port of the `sha1` reference package.
- `sum1(List<int>)` → `Uint8List` — 20-byte FIPS 180-4 digest.
- `hexString(List<int>)` → `String` — 40-char lowercase hex digest.
- `Sha1Digest` — streaming hasher with `update`, non-destructive `sum1` /
  `hexDigest`, and `cloneDigest` for independent snapshots.
- 32-bit wrap-around arithmetic on Dart's 64-bit `int` via `& 0xFFFFFFFF`
  masking and logical (`>>>`) shifts; big-endian block parsing, length, output.
- 23 unit tests: the FIPS 180-4 / RFC 3174 vectors (empty, "abc", 56-byte,
  one-million-'a'), output-format/determinism/avalanche properties, padding
  block-boundary edge cases (0/55/56/63/64/127/128), and streaming parity with
  the one-shot API (byte-at-a-time, block-split, clone independence).
- Documented security note: SHA-1 collision resistance is broken; checksum use
  only.
