# Changelog — coding_adventures_rpc

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-11

### Added

- **ErrorCodes module** — Standard RPC error code constants:
  `PARSE_ERROR` (-32700), `INVALID_REQUEST` (-32600),
  `METHOD_NOT_FOUND` (-32601), `INVALID_PARAMS` (-32602),
  `INTERNAL_ERROR` (-32603). Identical to JSON-RPC 2.0 but
  codec-agnostic.
- **RpcError class** — `StandardError` subclass carrying both a
  `code` (Integer) and a human-readable message. Raised by codecs
  and framers to signal protocol-level failures.
- **Message types** — Four `Struct` classes with `keyword_init: true`:
  - `RpcRequest`       — `id`, `method`, `params`
  - `RpcResponse`      — `id`, `result`
  - `RpcErrorResponse` — `id`, `code`, `message`, `data`
  - `RpcNotification`  — `method`, `params`
- **RpcCodec module** — Duck-typing interface contract for codecs.
  Documents `#encode(msg) → bytes` and `#decode(bytes) → RpcMessage`
  with detailed inline explanations and a skeleton implementation.
  Raises `NotImplementedError` from both methods when included without
  overriding.
- **RpcFramer module** — Duck-typing interface contract for framers.
  Documents `#read_frame → bytes|nil` and `#write_frame(bytes)`.
  Raises `NotImplementedError` from both methods when included without
  overriding.
- **Server class** — Codec/framer-driven read-dispatch-write loop:
  - `#initialize(codec, framer)` — accepts any codec+framer duck pair
  - `#on_request(method, &block)` — registers request handler; chainable
  - `#on_notification(method, &block)` — registers notification handler; chainable
  - `#serve` — blocking loop; `rescue Exception` for handler panic recovery;
    sends `-32603 Internal error` on handler raise; sends `-32601 Method not
    found` for unregistered request methods; silently drops unknown
    notifications; discards incoming responses (server-only mode)
- **Client class** — Synchronous request-response client:
  - `#initialize(codec, framer)` — accepts any codec+framer duck pair
  - `#request(method, params)` — auto-id, blocking, dispatches server-push
    notifications while waiting; raises `RpcError` on server error or
    connection closed
  - `#notify(method, params)` — fire-and-forget; returns nil immediately
  - `#on_notification(method, &block)` — registers server-push handler; chainable
- **Tests** — 38 minitest tests using `MockCodec` + `MockFramer` in-process
  doubles. Covers all spec test targets. Coverage exceeds 95%.
