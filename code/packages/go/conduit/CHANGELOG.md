# Changelog — conduit (Go)

All notable changes to the `conduit` Go package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-06-14

### Added

- Initial release — WEB14 port of the Conduit framework to Go via cgo.
- `conduit.go`: single-file cgo wrapper over the reusable `conduit-capi`
  C ABI (Rust staticlib from WEB12). No new FFI trust boundary introduced;
  header injection defence, status clamping, UTF-8 validation, and panic
  isolation are enforced by the Rust layer.
- Handler types: `HandlerFunc`, `BeforeFunc`, `AfterFunc`.
- Response helpers: `HTML`, `JSON`, `Text`, `Respond`, `Redirect` (with
  response-splitting guard on CR/LF in the location).
- `Request` accessors: `Method`, `Path`, `QueryString`, `ContentType`,
  `RemoteAddr`, `Error`, `Body`, `BodyString`, `Param`, `Query`, `Header`.
- `Application` DSL: `New`, `Get`, `Post`, `Put`, `Delete`, `Patch`, `Route`,
  `Before`, `After`, `NotFound`, `OnError`, `Set`, `GetSetting`, `Bind`, `Free`.
- `Server`: `Serve`, `ServeBackground`, `Stop`, `LocalPort`, `Running`, `Close`.
- `Halt(status, body)` for Sinatra-style non-local exits from handlers.
- `cgo.Handle` pattern for GC-safe closure registration (avoids `go vet`
  unsafe.Pointer warnings; closures survive GC while C holds a reference).
- `//export goConduitHandler/Before/After/Free` trampolines with `defer/recover`
  so no Go panic can unwind across the C boundary.
- C shims in the CGO preamble cast `uintptr` handles to `void*` ctx so Go
  only passes clean `C.uintptr_t` values (no cgo Rule 6 violations).
- Static linking by full `.a` path (not `-l conduit_capi`) so Linux `ld` cannot
  prefer the sibling cdylib and produce binaries that fail at runtime.
- 14 unit + E2E tests in `conduit_test.go`:
  - `TestHTMLDefaults`, `TestHelperStatusAndTypes`, `TestRedirect`, `TestSettings`,
    `TestChaining`, `TestBindReturnsPort` — unit tests of helpers and the DSL.
  - `TestEndToEnd` — 8 subtests over a live `ServeBackground()` server: root,
    route param, POST echo, query, before-halt, error handler, not-found, redirect.
  - 30-second watchdog via `time.AfterFunc` to prevent CI hangs.
