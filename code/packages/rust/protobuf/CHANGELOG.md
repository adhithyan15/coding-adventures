# Changelog — protobuf

## [0.1.0] — Unreleased

Initial release: a zero-dependency Protocol Buffers **wire-format** codec.

- LEB128 unsigned varint encode/decode with boundary + spec-byte tests
  (`varint 300 → [0xac, 0x02]`, `field 1 varint 150 → [0x08, 0x96, 0x01]`).
- Four wire types: `Varint` (0), `Fixed64` (1), `LengthDelimited` (2),
  `Fixed32` (5). Deprecated group types (3/4) are rejected as malformed.
- `Writer` — append `varint` / `bytes` / `string` / `message` / `fixed32` /
  `fixed64` fields in order; `into_bytes()` yields a complete message.
- `Reader` — iterate `(field number, Value)` with `next_field()`; unknown field
  numbers are yielded so callers can skip them (forward compatibility).
- Decode errors: truncated/over-long varint, unexpected EOF, unknown wire type,
  zero field number. Encoding is infallible.
- `#![forbid(unsafe_code)]`; no dependencies.

Purpose: replace the third-party `prost` crate in `engram-anki-package` for the
Anki `.apkg` `meta`/`media` protobuf messages, without a build-time `.proto`
code generator. First step of the Engram zero-dependency plan
(`code/specs/engram-zero-dep-plan.md`, Phase A).
