# Changelog — conduit-hello (TypeScript)

## [0.1.0] — 2026-04-25

### Added

Initial release — demo program for the WEB05 TypeScript Conduit port.

- `hello.ts` — 8-route Express-style demo: root HTML, named capture JSON,
  POST echo, 301 redirect, halt(403), 503 before filter, error handler, 404.
- `tests/conduit_hello.test.ts` — 15 integration tests via real TCP.
