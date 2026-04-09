# Changelog

## 0.1.0 — 2026-04-08

Initial release.

### Added
- `RespError` class with `message`, `error_type`, and `detail` properties; equality and hashing
- `RespValue` type alias covering all RESP2 Python representations
- `encode_simple_string(s)` — encodes `+<text>\r\n`; raises `ValueError` if text contains `\r` or `\n`
- `encode_error(msg)` — encodes `-<msg>\r\n`
- `encode_integer(n)` — encodes `:<n>\r\n`
- `encode_bulk_string(s)` — binary-safe length-prefixed encoding; `None` → `$-1\r\n`
- `encode_array(items)` — recursive array encoding; `None` → `*-1\r\n`
- `encode(value)` — high-level dispatcher: handles `None`, `bool`, `int`, `str`, `bytes`, `list`, `RespError`
- `decode(buffer)` — stateless pure-function parser returning `(RespValue, bytes_consumed)`; returns `(None, 0)` on incomplete input
- `decode_all(buffer)` — drain all complete messages from a buffer; returns `(messages, consumed)`
- `RespDecoder` — stateful streaming decoder with `feed()`, `has_message()`, `get_message()`, and `decode_all()` convenience method
- `RespDecodeError` — raised on syntactically invalid RESP bytes
- Inline command support: plain text lines (e.g. `PING\r\n`) parsed as arrays of bytes tokens
- 100% test coverage across 129 tests
- Literate comments explaining TCP framing, the read buffer pattern, and the recursive-descent parser state machine
