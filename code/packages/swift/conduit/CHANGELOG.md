# Changelog — Conduit (Swift)

## Unreleased

### Fixed

`BUILD_windows` now carries the `# build-tool: deps=rust/conduit-capi` declaration.
The build tool reads that directive out of whichever BUILD file it selects for the
current platform, so a directive present only in `BUILD` left the `conduit-capi` edge
missing from the dependency graph on Windows.

## [0.1.0] - 2026-06-13

### Added — WEB12 Swift Conduit port

- A Sinatra/Express-style web framework for Swift over the Rust web-core engine,
  hosted through the reusable `conduit-capi` C ABI via Swift's native C interop
  (systemLibrary + module map + compile-time static linking) — no third-party FFI.
- **DSL**: `Application` with `get/post/put/delete/patch`, `route`, `before`,
  `after` (transforming), `notFound`, `onError`, `set`/`getSetting`, `bind` —
  all chainable. Handlers are `(Request) throws -> Response`.
- **Response** helpers `html/json/text/respond/redirect`; `redirect` rejects
  CR/LF (scanning unicode scalars, since "\r\n" is one grapheme in Swift).
- **halt(...)** for Sinatra-style non-local exits (thrown `ConduitHalt`).
- **Request**: `method/path/queryString/body/contentType/remoteAddr/error`,
  `param/query/header`.
- **Server**: `serve` (foreground), `serveBackground`, `stop`, `localPort`,
  `running`.
- Closures are boxed and retained; a `ctx_free` trampoline releases them when the
  app/server is disposed (no leaks).

### Tests

20 Swift tests (Response helpers incl. native round-trip + CRLF guard; Application
DSL/settings/bind; a full end-to-end server driven by a POSIX-socket HTTP/1.0
client with a watchdog) plus the 5 conduit-capi Rust tests.

### Security

Header-injection defense, status clamping, and UTF-8 validation live in
`conduit-capi` (audited once for all C-ABI ports). `redirect` rejects CR/LF in
the location.
