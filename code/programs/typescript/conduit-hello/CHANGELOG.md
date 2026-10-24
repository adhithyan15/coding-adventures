# Changelog — conduit-hello (TypeScript)

## [Unreleased]

### Fixed

- Added a `BUILD_windows` skip stub. The program needs `typescript/conduit`'s
  native addon, which conduit's own `BUILD_windows` does not build on Windows,
  so its tests failed with `Cannot find module '../conduit_native_node.node'`.

## [0.1.0] — 2026-04-25

### Added

Initial release — demo program for the WEB05 TypeScript Conduit port.

- `hello.ts` — 8-route Express-style demo: root HTML, named capture JSON,
  POST echo, 301 redirect, halt(403), 503 before filter, error handler, 404.
- `tests/conduit_hello.test.ts` — 15 integration tests via real TCP.
