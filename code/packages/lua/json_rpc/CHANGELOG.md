# Changelog — coding-adventures-json-rpc (Lua)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-11

### Added

- `JsonRpc.MessageReader` — reads Content-Length-framed JSON-RPC messages from
  a byte stream (`read_message()`, `read_raw()`).
- `JsonRpc.MessageWriter` — writes Content-Length-framed messages to a byte
  stream (`write_message()`, `write_raw()`).
- `JsonRpc.Server` — request/notification dispatch loop over stdin/stdout.
  - `on_request(method, handler)` — register a request handler (chainable).
  - `on_notification(method, handler)` — register a notification handler (chainable).
  - `serve()` — blocking read-dispatch-write loop.
- Message constructors: `Request`, `Response`, `ErrorResponse`, `Notification`.
- Standard error-code constants in `JsonRpc.errors` (`PARSE_ERROR`, `INVALID_REQUEST`,
  `METHOD_NOT_FOUND`, `INVALID_PARAMS`, `INTERNAL_ERROR`).
- Minimal inline JSON encoder/decoder (no external dependencies) supporting
  objects, arrays, strings, numbers, booleans, and null — sufficient for all
  JSON-RPC message shapes.
- 25+ busted unit tests covering framing, parsing, round-trips, and dispatch.

### Notes

- No dependency on any other coding-adventures package — pure Lua stdlib only.
- The inline JSON codec is intentionally minimal; it handles the JSON-RPC message
  shapes described in the spec and nothing more.  For general-purpose JSON work,
  use `coding-adventures-json-serializer`.
