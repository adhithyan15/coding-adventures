# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `resp-protocol` crate: RESP v2 (the Redis wire
  protocol) — the recursive value model, encoder, decoder, and streaming
  decoder.
- `RespValue` tagged union (simple string, error, integer, bulk string, array;
  with null bulk/array), built via `resp_*` constructors and freed recursively
  with `resp_free`. `resp_equal` for structural comparison; `resp_error_type` /
  `resp_error_detail` for the error-message split.
- `resp_encode` (→ `RESP_ENCODE_OK` / `..._ERR_SIMPLE_NEWLINE` / `..._ERR_ALLOC`)
  and `resp_decode` / `resp_decode_all` (→ `RESP_DECODE_OK` with `*consumed` /
  `..._INCOMPLETE` / `..._ERROR`).
- Streaming `RespDecoder`: `resp_decoder_new`/`_free`/`_feed`/`_has_message`/
  `_get_message`/`_decode_all`/`_has_error`, accumulating bytes across feeds and
  latching an error on a malformed frame.
- Strict signed-decimal parsing with overflow rejection, a validating UTF-8 scan
  for text frames, and overflow-guarded allocations (`calloc` checked multiply
  for arrays, guarded doubling for the encode buffer).
- Tests use the crate's own vectors (encode/decode of every frame type, nested
  arrays, error cases, incomplete inputs, invalid UTF-8, `decode_all`, streaming)
  under GCC and Clang via `iso-harness`.
