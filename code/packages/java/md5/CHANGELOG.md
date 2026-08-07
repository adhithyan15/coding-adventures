# Changelog — md5 (Java)

## 0.1.0 — 2026-07-11

### Added

- Initial release: pure-Java port of the `md5` reference package.
- `Md5.sumMd5(byte[])` → 16-byte RFC 1321 digest.
- `Md5.hexString(byte[])` → 32-char lowercase hex digest.
- `Md5.Digest` — streaming hasher with `update`, non-destructive `digest` /
  `hexDigest`, and `copy` for independent snapshots.
- Correct little-endian block parsing, length field, and digest output; Java's
  native 32-bit `int` arithmetic (`Integer.rotateLeft`, `& 0xff` byte masking).
- 21 JUnit tests: all seven RFC 1321 Appendix A.5 vectors, little-endian byte
  order + the 0x00..0xFF known digest, output-format/determinism/avalanche
  properties, padding block boundaries (0/55/56/63/64/127/128), and streaming
  parity with the one-shot API.
- Documented security note: MD5 is cryptographically broken; checksum use only.
