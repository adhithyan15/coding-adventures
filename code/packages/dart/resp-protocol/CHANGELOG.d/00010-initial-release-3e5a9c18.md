## 0.1.0

- Initial Dart lane of the resp-protocol identity (closes 15/15): five
  RESP2 value types with deep equality, total encoding, incremental
  decoding (null on truncation, RespDecodeError on malformation),
  pipelined decodeAll, and a streaming RespDecoder.
- Strict integer/length parsing (shape-gated, 64-bit ranged) matching the
  strict lanes; payload bytes always copied out of the input buffer.
- Hardening: overflow-safe bulk bounds, 128-deep nesting cap, UTF-8
  failures as RespDecodeError, copy-on-construct arrays.
- 25 tests: wire-exact battery, nulls/nesting/binary safety, exhaustive
  prefix truncation, malformed battery, security regressions, and a
  512-case seeded round-trip sweep; 95% branch+function coverage gate.
