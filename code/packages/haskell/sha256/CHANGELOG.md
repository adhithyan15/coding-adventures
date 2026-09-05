# Changelog

## 0.2.0

- Added an opaque immutable incremental SHA-256 context over strict
  `ByteString` chunks.
- Added exact chunk updates, repeatable byte and lowercase-hex finalization,
  and O(1) immutable context copies while preserving both one-shot APIs.
- Added FIPS, binary-byte, padding-boundary, 8 KiB split, byte-at-a-time,
  repeated-finalization, branching, and million-byte regressions.
- Enforced the FIPS message-length domain with checked counter arithmetic and
  explicit boundary tests so the encoded 64-bit bit length cannot wrap.
- Replaced the stale Windows BUILD skip with a real Cabal test front door.

## 0.1.0

- Initial scaffold.
