# Changelog — sha1 (Java)

## 0.1.0 — 2026-07-11

### Added

- Initial release: pure-Java port of the `sha1` reference package, completing
  the common hash family in Java (alongside sha256 and md5).
- `Sha1.sum1(byte[])` → 20-byte FIPS 180-4 digest.
- `Sha1.hexString(byte[])` → 40-char lowercase hex digest.
- `Sha1.Digest` — streaming hasher with `update`, non-destructive `digest` /
  `hexDigest`, and `copy` for independent snapshots.
- Big-endian block parsing/length/output; Java's native 32-bit `int` arithmetic
  (`Integer.rotateLeft`, `& 0xff` byte masking).
- 20 JUnit tests: FIPS 180-4 / RFC 3174 vectors (empty, "abc", 56-byte,
  one-million-'a'), output-format/determinism/avalanche, padding block
  boundaries (0/55/56/63/64/127/128), and streaming parity with the one-shot API.
- Documented security note: SHA-1 collision resistance is broken; checksum use only.
