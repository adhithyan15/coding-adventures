# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `protobuf` crate: a
  zero-dependency Protocol Buffers wire-format codec in namespace
  `ca::protobuf` (encode/decode only, no `.proto` compiler).
- `Writer` — chainable `varint` / `bytes` / `string` / `message` / `fixed32` /
  `fixed64` / `write_varint` over a `std::vector<std::uint8_t>`; `into_bytes()`
  moves the buffer out.
- `Reader` — `next_field()` returns `std::optional<Field>` (`nullopt` at end)
  and throws `ca::protobuf::Error` (carrying an `ErrorKind`) on malformed input;
  unknown field numbers are yielded for forward compatibility. `Value` is tagged
  by `WireType` with `as_varint()` / `as_bytes()` (`std::optional`) and value
  equality; length-delimited payloads borrow the input via a hand-rolled
  `ByteView` (avoids the non-standard `char_traits<unsigned char>`).
- 47 checks mirroring the crate's known-answer vectors (spec byte sequences,
  all wire types, nested messages, unknown-field skipping, every error path),
  run under every ISO C++ compiler via the shared `iso-harness`; also clean
  under ASan + UBSan.
