# Changelog — @coding-adventures/rpc

All notable changes to this package will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This package uses [Semantic Versioning](https://semver.org/).

---

## [0.1.0] — 2026-04-11

### Added

- **`RpcCodec<V>` interface** (`src/codec.ts`) — translates between `RpcMessage<V>` and `Uint8Array`. Stateless; a single instance can be shared across encode/decode calls.
- **`RpcFramer` interface** (`src/framer.ts`) — reads and writes discrete byte frames from a raw byte stream. `readFrame()` returns `null` on clean EOF.
- **`RpcMessage<V>` discriminated union** (`src/message.ts`) — covers all four RPC message shapes: `RpcRequest<V>`, `RpcResponse<V>`, `RpcErrorResponse<V>`, `RpcNotification<V>`. The `kind` field is the TypeScript discriminant.
- **`RpcId` type alias** — `string | number`; the correlation key that ties requests to responses.
- **`RpcErrorCodes` constants** (`src/errors.ts`) — codec-agnostic standard error codes: `ParseError` (-32700), `InvalidRequest` (-32600), `MethodNotFound` (-32601), `InvalidParams` (-32602), `InternalError` (-32603).
- **`RpcError` class** — thrown by codec/framer implementations to signal fatal message problems. Carries a `code` field.
- **`RpcServer<V>` class** (`src/server.ts`) — codec-agnostic request dispatcher. Fluent `onRequest()` / `onNotification()` registration. Synchronous blocking `serve()` loop with panic recovery (handler exceptions become `-32603` responses). Unknown notifications are silently dropped per spec.
- **`RpcClient<V>` class** (`src/client.ts`) — sends requests and blocks for matching responses. Auto-incrementing request ids starting at 1. Server-push notification dispatch via `onNotification()`. `RpcClientError` thrown on server errors or EOF.
- **`RpcClientError` class** — thrown by `RpcClient.request()` when the server responds with an error or the connection closes before a response arrives.
- **`src/index.ts`** — re-exports all public types and classes.
- **Comprehensive test suite** (`tests/rpc.test.ts`) — in-memory `MockCodec` (JSON under the hood) and `MockFramer` (Buffer queue). 50+ test cases covering server dispatch, client request/response correlation, error paths, notification handling, edge cases.
