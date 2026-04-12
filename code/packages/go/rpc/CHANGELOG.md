# Changelog

All notable changes to the `rpc` Go package are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-11

### Added

- `RpcMessage[V]` — sealed sum-type interface representing the four RPC message variants.
- `RpcRequest[V]` — request from client to server, carrying `Id`, `Method`, and `Params *V`.
- `RpcResponse[V]` — success reply from server to client, carrying `Id` and `Result *V`.
- `RpcErrorResponse[V]` — error reply carrying `Id`, `Code`, `Message`, and `Data *V`. Implements `error` interface.
- `RpcNotification[V]` — fire-and-forget one-way event with no response.
- `RpcId` — type alias for `any`, representing string or integer correlation ids.
- `RpcCodec[V]` — interface for pluggable message serialization (`Encode` / `Decode`).
- `RpcFramer` — interface for pluggable byte-stream framing (`ReadFrame` / `WriteFrame`).
- `RpcServer[V]` — read-dispatch-write loop with method handler registry and panic recovery.
  - `NewRpcServer[V](codec, framer)` constructor.
  - `OnRequest(method, handler)` — register request handler (chainable).
  - `OnNotification(method, handler)` — register notification handler (chainable).
  - `Serve()` — blocking loop; exits on clean EOF; recovers from handler panics.
- `RpcClient[V]` — blocking request/response correlation with server-push notification dispatch.
  - `NewRpcClient[V](codec, framer)` constructor.
  - `Request(method, params)` — blocking call returning `(*V, *RpcErrorResponse[V], error)`.
  - `Notify(method, params)` — fire-and-forget notification.
  - `OnNotification(method, handler)` — register server-push handler (chainable).
- Standard error code constants: `ParseError`, `InvalidRequest`, `MethodNotFound`, `InvalidParams`, `InternalError`.
- `EOF` — convenience re-export of `io.EOF` so callers comparing `ReadFrame` errors need not import `io`.
- `rpc_test.go` — 20 tests covering all spec-required scenarios via mock codec and framer.
  - Server: dispatch, MethodNotFound, handler error, panic recovery, notification dispatch, unknown notification drop, decode error, multiple sequential requests, nil params.
  - Client: request success, error response, connection closed, notify, server-push notification during wait, auto-incrementing ids.
  - Chaining: `OnRequest`/`OnNotification` return server/client for method chaining.
  - Error codes: constant value verification.
  - `rpc.EOF` identity check.
- `go.mod` with module path `github.com/coding-adventures/rpc`, Go 1.23, stdlib only.
- `BUILD` with `go test ./... -v -cover`.
- `README.md` with architecture diagram, usage examples, and interface documentation.

### Coverage

89.2% statement coverage (exceeds the 80% minimum).

### Notes

- No imports outside stdlib in the main package files.
- `rpc_test.go` uses `encoding/json` only in the mock codec, which is fine for test code — the production rpc package never imports it.
- The `V` type parameter is intentionally unconstrained (`any`) so the package works with all codec value types.
- `RpcId = any` is a type alias (not a new type) to avoid conversions at call sites.
