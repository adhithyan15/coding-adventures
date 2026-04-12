# Changelog

All notable changes to `coding_adventures_json_rpc` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — 2026-04-11

### Added

- `ErrorCodes` module with standard JSON-RPC 2.0 constants:
  `PARSE_ERROR (-32700)`, `INVALID_REQUEST (-32600)`,
  `METHOD_NOT_FOUND (-32601)`, `INVALID_PARAMS (-32602)`,
  `INTERNAL_ERROR (-32603)`.
- `Error < StandardError` — carries a numeric `code` alongside `message`.
- `Request`, `Notification`, `Response`, `ResponseError` — immutable
  `Data.define` value objects for all four JSON-RPC message shapes.
- `CodingAdventures::JsonRpc.parse_message(hash)` — validates a Hash from
  `JSON.parse` and returns a typed message, raising `Error` on invalid input.
- `CodingAdventures::JsonRpc.message_to_h(msg)` — converts a typed message
  back to a plain Hash for `JSON.generate`, adding `"jsonrpc": "2.0"`.
- `MessageReader` class — reads Content-Length-framed messages from any IO.
  - `read_message` → typed Message or nil on EOF
  - `read_raw` → raw JSON String or nil on EOF
- `MessageWriter` class — writes Content-Length-framed messages to any IO.
  - `write_message(msg)` — serialises a typed message
  - `write_raw(json)` — frames a pre-serialised string
- `Server` class — combines reader + writer with a dispatch table.
  - `on_request(method) { |id, params| ... }` — chainable
  - `on_notification(method) { |params| ... }` — chainable
  - `serve` — blocking read-dispatch-write loop
- 44 Minitest test cases covering all components, including round-trip
  tests and all error paths.
