# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `protobuf` crate: a zero-dependency Protocol
  Buffers wire-format codec (encode/decode only, no `.proto` compiler).
- `PbWriter` — a malloc-owned growable byte buffer with `pb_write_varint`,
  `pb_varint`, `pb_bytes`, `pb_string`, `pb_message`, `pb_fixed32`,
  `pb_fixed64`; `pb_writer_bytes`/`_len` to borrow and `pb_writer_take` for the
  `into_bytes` ownership transfer. Capacity doubling is guarded against
  `size_t` overflow and latches an `oom` flag on allocation failure.
- `PbReader` — a non-allocating cursor; `pb_reader_next_field` decodes varint /
  fixed32 / fixed64 / length-delimited fields (payloads borrow the input),
  yields unknown field numbers, and reports `PbError`
  (`PB_ERR_TRUNCATED_VARINT`, `PB_ERR_UNEXPECTED_EOF`,
  `PB_ERR_UNKNOWN_WIRE_TYPE`, `PB_ERR_ZERO_FIELD_NUMBER`) plus
  `pb_error_message`. `pb_value_as_varint` / `pb_value_as_bytes` accessors.
- 81 checks mirroring the crate's known-answer vectors (the canonical
  `varint 300 → AC 02` and `field 1 varint 150 → 08 96 01`, all wire types,
  nested messages, unknown-field skipping, and every error path), run under
  every ISO C compiler via the shared `iso-harness`; also clean under
  ASan + UBSan.
