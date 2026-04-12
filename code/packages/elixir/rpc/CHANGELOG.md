# Changelog

All notable changes to the `rpc` Elixir package are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-11

### Added

- **`Rpc.Message`** — Codec-agnostic message structs:
  - `Rpc.Message.Request` — request with `id`, `method`, `params`
  - `Rpc.Message.Response` — successful response with `id`, `result`
  - `Rpc.Message.ErrorResponse` — error response with `id`, `code`, `message`, `data`
  - `Rpc.Message.Notification` — fire-and-forget with `method`, `params`
  - `Rpc.Message.t()` union type alias

- **`Rpc.Codec`** — Behaviour for translating between `Rpc.Message.t()` and bytes:
  - `encode/1` callback: `Rpc.Message.t() → {:ok, binary()} | {:error, term()}`
  - `decode/1` callback: `binary() → {:ok, Rpc.Message.t()} | {:error, Rpc.Message.ErrorResponse.t()}`

- **`Rpc.Framer`** — Behaviour for splitting a byte stream into discrete frames:
  - `read_frame/1` callback: stateful read returning `{:ok, binary(), state}`, `:eof`, or `{:error, term()}`
  - `write_frame/2` callback: stateful write returning `{:ok, state}` or `{:error, term()}`

- **`Rpc.Errors`** — Standard error code constants and constructor helpers:
  - `parse_error/0` → `-32700`
  - `invalid_request/0` → `-32600`
  - `method_not_found/0` → `-32601`
  - `invalid_params/0` → `-32602`
  - `internal_error/0` → `-32603`
  - `make_parse_error/1`, `make_invalid_request/1`, `make_method_not_found/1`,
    `make_invalid_params/1`, `make_internal_error/1`

- **`Rpc.Server`** — Blocking codec-agnostic server dispatch loop:
  - `serve/4` — reads frames, decodes messages, dispatches handlers, writes responses
  - `register_request/3` — register a `fn(id, params) → result` handler
  - `register_notification/3` — register a `fn(params) → :ok` handler
  - Exception recovery via `try/rescue` — handler panics become `-32603` responses
  - Unknown notifications silently dropped per RPC spec

- **`Rpc.Client`** — Blocking codec-agnostic RPC client:
  - `new/3` — construct from codec module, framer module, initial framer state
  - `request/3` — send request, block for matching response (by id), return result
  - `notify/3` — fire-and-forget notification
  - `on_notification/3` — register handler for server-push notifications
  - Auto-incrementing request ids starting at 1

- **`Rpc`** — Top-level module with convenience delegates to `Rpc.Server`

- **Tests** — 62 ExUnit tests with in-memory `MockCodec` (Erlang term serialization)
  and `SpyFramer` (Agent-backed frame capture). Coverage targets >80%.

- **No Hex dependencies** — stdlib and OTP only.
