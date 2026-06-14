# Changelog — conduit (Haskell)

## 0.1.0 — 2026-06-14

Initial release (WEB18).

### Added
- `Conduit.FFI` — raw GHC FFI declarations for all `conduit-capi` symbols:
  phantom handle types (`CApp`, `CServer`, `CRequest`, `CResponse`),
  callback type aliases (`HandlerFn`, `AfterFn`, `CtxFreeFn`),
  `"wrapper"` imports (`mkHandler`, `mkAfter`, `mkCtxFree`), and all
  `foreign import ccall` bindings.
- `Conduit.Request` — `Request` record with eagerly-read fields plus lazy
  `reqParam` / `reqQuery` / `reqHeader` IO accessors.
- `Conduit.Response` — `Response` record, `ConduitHalt` exception, and
  builder helpers: `respond`, `html`, `json`, `textPlain`, `redirect`,
  `halt`.  `redirect` validates CR/LF absence.
- `Conduit.App` — `Application` type, `StablePtr`/`FunPtr` closure boxing,
  trampolines with full exception handling (`ConduitHalt` + `SomeException`),
  and the full DSL: `newApplication`, `addRoute`, `get/post/put/delete/patch/options`,
  `before`, `after`, `notFound`, `onError`, `setSetting`, `getSetting`, `bind`.
- `Conduit.Server` — `Server` type, `serve` (`safe` FFI, blocking),
  `serveBackground`, `stop`, `localPort`, `running`, `freeServer`.
- `Conduit` — re-export module covering the full public API.
- Test suite:
  - `ConduitSpec` — 9 unit tests for response helpers, redirect guard,
    halt exception, and settings round-trip.
  - `ServerE2ESpec` — 10 E2E tests over raw HTTP/1.0 sockets on ephemeral
    port 0: root, named param, POST echo, query string, redirect, halt,
    before-filter, on_error, notFound.
- `BUILD` / `BUILD_windows` — `sh tools/run-tests.sh` on Unix; echo skip on
  Windows (cdylib cross-compile out of scope).
- `tools/run-tests.sh` — builds `conduit-capi` with cargo then runs
  `cabal test --extra-lib-dirs`.
- `cabal.project` — single-package project file (not a wildcard, as per
  lessons.md: cabal does not discover sibling deps from sibling project files).
