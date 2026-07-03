# Changelog — conduit-hello (Go)

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
