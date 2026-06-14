# Changelog — coding_adventures_conduit

## 0.1.0 — 2026-06-14

Initial release (WEB17).

- Dart 3 FFI binding (`dart:ffi`) for the `conduit-capi` Rust cdylib (WEB12).
- Fluent object-oriented API: `Application().get('/hello', h).bind('127.0.0.1', 3000)`.
- `Response` class: `html`, `json`, `text`, `redirect`, `respond`, `withStatus`, `withHeader`.
  - Dart optional named parameters work natively; `html(body, status: 201)` is idiomatic.
- `Request` class: `method`, `path`, `queryString`, `contentType`, `remoteAddr`, `error`,
  `param`, `query`, `header`, `body`, `bodyString`.
- `Application` builder: `set`, `getSetting`, `get`, `post`, `put`, `delete`, `patch`, `route`,
  `before`, `after`, `notFound`, `onError`, `bind`, `dispose`.
- `Server`: `localPort`, `isRunning`, `serveBackground`, `serve` (async), `stop`, `dispose`.
- `HaltException` for non-local exits from handlers.
- `NativeCallable.isolateLocal` trampolines — one static callable per callback kind;
  cross-thread safe (Rust/Tokio thread posts to Dart event loop, blocking until done).
- Global integer registry replaces GCHandle — `allocHandler/allocBefore/allocAfter` assign
  monotonically-increasing integer keys; `ctx_free` releases the entry.
- 40 tests across 4 groups; watchdog timer prevents CI hangs.

Security properties:
- `CONDUIT_CAPI_PATH` rejected if not absolute — prevents path-traversal via relative paths.
- `sanitizeForLog` strips control characters from native-sourced strings before stderr writes.
- `X-Content-Type-Options: nosniff` stamped on every response in conduit-hello.
- CRLF guard on `Response.redirect` location.
- Status code range validation `[100, 999]` before uint16 cast.
- Bounds check before `Uint8List.fromList` on native body lengths.
