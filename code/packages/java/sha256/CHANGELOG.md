# Changelog — sha256 (Java)

## 0.1.0 — 2026-07-11

### Added

- Initial release: pure-Java port of the `sha256` reference package — the first
  language beyond Dart in the pure-port + native campaign.
- `Sha256.sha256(byte[])` → 32-byte FIPS 180-4 digest.
- `Sha256.sha256Hex(byte[])` → 64-char lowercase hex digest.
- `Sha256.Hasher` — streaming hasher with `update`, non-destructive `digest` /
  `hexDigest`, and `copy` for independent snapshots.
- Uses Java's native 32-bit `int` arithmetic (`>>>`, `Integer.rotateRight`) — no
  masking required.
- 22 JUnit tests: the four FIPS 180-4 vectors (incl. one-million-'a'),
  output-format/determinism/avalanche properties, padding/block-boundary edge
  cases (55/56/63/64/127/128), and streaming parity with the one-shot API.
