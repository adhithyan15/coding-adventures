# Changelog — CodingAdventures::Rpc

All notable changes to this package will be documented here.

Format: [Semantic Versioning](https://semver.org/). Sections: Added, Changed,
Deprecated, Removed, Fixed, Security.

---

## [0.01] — 2026-04-11

### Added

- **`CodingAdventures::Rpc::Errors`** — integer constants for the five
  standard RPC error codes (`PARSE_ERROR`, `INVALID_REQUEST`,
  `METHOD_NOT_FOUND`, `INVALID_PARAMS`, `INTERNAL_ERROR`). Matches the
  same codes used by `CodingAdventures::JsonRpc::Errors` but is now the
  canonical source of truth for the rpc layer.

- **`CodingAdventures::Rpc::Message`** — constructor functions for all four
  RPC message kinds, returned as blessed hashrefs with a `kind` discriminant:
  - `make_request($id, $method, $params)`
  - `make_response($id, $result)`
  - `make_error($id, $code, $message [, $data])`
  - `make_notification($method [, $params])`

- **`CodingAdventures::Rpc::Codec`** — abstract base class documenting the
  codec interface contract (`encode($msg)` / `decode($bytes)`). Base methods
  die with helpful messages if a subclass forgets to override them.

- **`CodingAdventures::Rpc::Framer`** — abstract base class documenting the
  framer interface contract (`read_frame()` / `write_frame($bytes)`). Same
  defensive base-method pattern as Codec.

- **`CodingAdventures::Rpc::Server`** — codec-agnostic dispatch server:
  - `new(codec => ..., framer => ...)` — validates required arguments
  - `on_request($method, $handler)` — chainable; replaces earlier registration
  - `on_notification($method, $handler)` — chainable
  - `serve()` — blocking read-dispatch-write loop with `eval{}` panic recovery
  - Unregistered methods → `-32601 Method not found`
  - Handler die → `-32603 Internal error` (server keeps running)
  - Codec parse failure → `-32700 Parse error` with null id
  - Codec shape failure → `-32600 Invalid Request` with null id
  - Framing errors → `-32600 Invalid Request` with null id
  - Notification handler panics are swallowed silently (no response written)
  - Incoming response messages are silently dropped (pure-server mode)

- **`CodingAdventures::Rpc::Client`** — synchronous RPC client:
  - `new(codec => ..., framer => ...)` — validates required arguments
  - `request($method [, $params])` — sends request, blocks for matching
    response, handles interleaved server-push notifications
  - `notify($method [, $params])` — fire-and-forget, no response expected
  - `on_notification($method, $handler)` — chainable; dispatched during
    `request()` for server-push notifications
  - Request ids are monotonically increasing integers starting at 1

- **`CodingAdventures::Rpc`** — main module; loads all sub-modules in
  leaf-to-root order (Errors, Message, Codec, Framer, Server, Client).

- **`t/rpc.t`** — 29 subtests with Test2::V0, using in-memory mock codec
  (pipe-delimited format) and mock framer (arrayref-backed). Covers:
  module loads, error constants, all message constructors, MockCodec
  round-trips, MockFramer operations, all server dispatch paths, client
  request/notify/on_notification, server-push notification dispatch,
  monotonic id generation, abstract base class die messages, constructor
  argument validation.

- `Makefile.PL`, `cpanfile`, `BUILD`, `BUILD_windows` — standard package
  scaffolding following the `json-rpc` package conventions.
