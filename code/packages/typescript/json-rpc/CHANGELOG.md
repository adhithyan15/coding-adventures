# Changelog

All notable changes to `@coding-adventures/json-rpc` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — 2026-04-11

### Added

- `ErrorCodes` object with standard JSON-RPC 2.0 error codes:
  `ParseError (-32700)`, `InvalidRequest (-32600)`, `MethodNotFound (-32601)`,
  `InvalidParams (-32602)`, `InternalError (-32603)`.
- `Request`, `Notification`, `Response`, `ResponseError` interfaces with a
  `type` discriminant for TypeScript exhaustiveness checking.
- `parseMessage(data: unknown): Message` — validates a raw JSON object and
  returns a typed `Message`, throwing `JsonRpcError` on invalid input.
- `messageToObject(msg: Message): Record<string, unknown>` — converts a typed
  message back to a plain object suitable for `JSON.stringify`, adding the
  `"jsonrpc": "2.0"` field.
- `JsonRpcError` — carries a numeric `code` alongside the standard `message`.
- `MessageReader` class — reads Content-Length-framed messages from a
  `Readable` stream using a Promise-based pull loop.
  - `readMessage(): Promise<Message | null>` — parses the payload.
  - `readRaw(): Promise<string | null>` — returns the raw JSON string.
- `MessageWriter` class — writes Content-Length-framed messages to a
  `Writable` stream.
  - `writeMessage(msg: Message): void`
  - `writeRaw(json: string): void`
- `Server` class — combines `MessageReader` and `MessageWriter` with a method
  dispatch table.
  - `onRequest(method, handler): this` — chainable.
  - `onNotification(method, handler): this` — chainable.
  - `serve(): Promise<void>` — blocking read-dispatch-write loop.
- 47 Vitest test cases covering all components, including round-trip tests
  and error path coverage.
