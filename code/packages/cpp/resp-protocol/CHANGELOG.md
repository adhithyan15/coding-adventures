# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `resp-protocol` crate, in
  namespace `ca::resp`: RESP v2 (the Redis wire protocol) — the recursive value
  model, encoder, decoder, and streaming decoder.
- `Value` with value semantics and factories (`simple_string`, `make_error`,
  `make_integer`, `bulk_string`, `bulk_string_null`, `make_array`,
  `make_array_null`); `operator==`; error split via `error_type()` /
  `error_detail()`. The recursive Array uses `std::vector<Value>` (incomplete
  type support, guaranteed since C++17).
- `encode` → `std::optional<std::vector<std::uint8_t>>` (nullopt iff a simple
  string contained CR/LF). `decode` → `DecodeResult`
  (value / incomplete / error); `decode_all` → `DecodeAllResult`.
- Streaming `Decoder`: `feed`, `has_message`, `get_message` →
  `std::optional<Value>`, `decode_all`, `has_error`.
- Strict signed-decimal parsing with overflow rejection and a validating UTF-8
  scan for text frames.
- Tests use the crate's own vectors (encode/decode of every frame type, nested
  arrays, error cases, incomplete inputs, invalid UTF-8, `decode_all`, streaming)
  under GCC and Clang via `iso-harness`.
