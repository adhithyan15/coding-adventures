# Changelog — coding-adventures-rpc

All notable changes to this crate will be documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-11

### Added

- `src/errors.rs` — `RpcError` infrastructure error type and five standard
  error code constants: `PARSE_ERROR` (-32700), `INVALID_REQUEST` (-32600),
  `METHOD_NOT_FOUND` (-32601), `INVALID_PARAMS` (-32602), `INTERNAL_ERROR`
  (-32603).

- `src/message.rs` — codec-agnostic message types parameterised by `V`:
  - `RpcId` type alias (`serde_json::Value`) covering String | Number | Null.
  - `RpcRequest<V>` — id + method + optional params.
  - `RpcResponse<V>` — id + result.
  - `RpcErrorResponse<V>` — optional id + code + message + optional data.
  - `RpcNotification<V>` — method + optional params.
  - `RpcMessage<V>` — discriminated union of all four.

- `src/codec.rs` — `RpcCodec<V>` trait with `encode` and `decode` methods.

- `src/framer.rs` — `RpcFramer` trait with `read_frame` and `write_frame`
  methods. Returning `None` from `read_frame` signals clean EOF.

- `src/server.rs` — `RpcServer<R, W, V>` with:
  - `new(codec, framer)` — construct with boxed trait objects.
  - `on_request(method, handler) -> &mut Self` — chainable handler registration.
  - `on_notification(method, handler) -> &mut Self` — chainable.
  - `serve()` — blocking read-dispatch-write loop with `catch_unwind` panic
    recovery for all handlers.

- `src/client.rs` — `RpcClient<V>` with:
  - `new(codec, framer)` — construct with boxed trait objects.
  - `on_notification(method, handler) -> &mut Self` — server-push notification
    handler registration.
  - `request(method, params) -> Result<V, RpcErrorResponse<V>>` — synchronous
    blocking request with auto-generated monotonically increasing ids.
  - `notify(method, params) -> Result<(), RpcError>` — fire-and-forget.

- `tests/integration_tests.rs` — 29 integration tests covering:
  - Server: dispatch, METHOD_NOT_FOUND, handler errors, panic recovery,
    continued operation after panic, notification dispatch, unknown notification
    silent drop, notification handler panic recovery, decode error → PARSE_ERROR,
    multiple sequential requests, method chaining.
  - Client: result return, error propagation, frame encoding, monotonic ids,
    notify sends frame, EOF before response, server-push notification while
    waiting.
  - Codec: round-trips for all four message types, parse error, invalid request.
  - Framer: write/read round-trip, EOF returns None, multiple frames.
  - Error types: `RpcError` message, display, error code constants.
