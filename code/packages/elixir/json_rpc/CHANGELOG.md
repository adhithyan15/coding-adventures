# Changelog — coding_adventures_json_rpc

## [0.1.0] — 2026-04-11

### Added

- `CodingAdventures.JsonRpc.Message` — `%Request{}`, `%Response{}`, `%Notification{}` structs with `parse_message/1` and `message_to_map/1`
- `CodingAdventures.JsonRpc.Reader` — `MessageReader` with Content-Length header parsing, reads from any Erlang I/O device
- `CodingAdventures.JsonRpc.Writer` — `MessageWriter` with Content-Length framing, writes to any Erlang I/O device
- `CodingAdventures.JsonRpc.Server` — blocking dispatch loop with `on_request/3` and `on_notification/3` (chainable)
- `CodingAdventures.JsonRpc.Errors` — standard error code constants (`-32700` through `-32603`) and `make_*` constructors
- `CodingAdventures.JsonRpc.JsonCodec` — internal JSON encoder/decoder using OTP 27 `:json` module when available, hand-written fallback for OTP < 27
- `CodingAdventures.JsonRpc` — top-level module with convenience delegators
- 57 ExUnit tests covering all components
- Zero external dependencies (stdlib only)
