# Changelog — conduit-hello (Go)

## [Unreleased]

### Added

- **Real Windows recipe.** `BUILD_windows` was an echo-skip; it now runs
  `tools\run-tests.ps1`, which builds the GNU-ABI Rust `conduit-capi`
  staticlib, puts MinGW-w64 gcc on PATH and runs `go test ./... -v` (the
  smoke test), mirroring the Unix recipe. The cgo link line is in the
  `go/conduit` package.

## [0.1.0] — 2026-06-14

### Added

- Initial release — demonstration web app for the Go Conduit framework (WEB14).
- Routes: home page, `/hello/:name`, `/api/echo`, `/api/query`, `/api/panic`,
  `/api/halt`, `/maintenance`.
- Before-filter that logs requests and intercepts `/maintenance` with a 503.
- After-hook that stamps an `x-served-by` header on every response.
- Custom `OnError` and `NotFound` handlers.
- `buildApp()` helper separated from `main()` to enable smoke testing without
  binding a fixed port.
- `TestSmoke` — 8-subtest E2E smoke test with a 30 s watchdog.
