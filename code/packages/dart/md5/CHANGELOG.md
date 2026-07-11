# Changelog — coding_adventures_md5

## 0.1.0 — 2026-07-11

### Added

- Initial release: pure-Dart port of the `md5` reference package.
- `sumMd5(List<int>)` → `Uint8List` — 16-byte RFC 1321 digest.
- `hexString(List<int>)` → `String` — 32-char lowercase hex digest.
- `Md5Digest` — streaming hasher with `update`, non-destructive `sumMd5` /
  `hexDigest`, and `cloneDigest` for independent snapshots.
- Correct little-endian block parsing, length field, and digest output, plus
  32-bit wrap-around arithmetic on Dart's 64-bit `int` via `& 0xFFFFFFFF`.
- 28 unit tests: all seven RFC 1321 Appendix A.5 vectors, little-endian byte
  order and the 0x00..0xFF known digest, output-format/determinism/avalanche
  properties, padding/block-boundary edge cases (0/55/56/63/64/127/128), and
  streaming parity with the one-shot API (byte-at-a-time, block-split, clone
  independence, one-million-'a').
