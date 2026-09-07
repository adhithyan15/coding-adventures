# Changelog

## 0.2.1

- Replaced the boxed, repeatedly rotated 64-word compression schedule with a
  reusable 16-word unboxed rolling schedule.
- Process complete caller blocks directly and copy only a bounded bridge block
  plus the terminal sub-block remainder during incremental updates.
- Added an optimized RTS-statistics allocation gate for the million-byte FIPS
  vector in one-chunk and 8 KiB streaming modes, each capped at 128 MiB.

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
