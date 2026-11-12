# Changelog — conduit-hello (Dart)

## Unreleased

- Windows now builds conduit-capi (shared helper
  `code/scripts/stage-conduit-capi-windows.ps1`) and conduit-dart-bridge, sets
  `CONDUIT_CAPI_PATH` and `CONDUIT_DART_BRIDGE_PATH` to the staged absolute DLL
  paths, and runs the same dart tests as the Unix recipe, instead of skipping
  (`BUILD_windows` + `tools/run-tests.ps1`).

## 0.1.0 — 2026-06-14

Initial release (WEB17).

- Demo program showing idiomatic Dart usage of `coding_adventures_conduit`.
- API-key before-filter, after-hook metadata stamping, route params, query string, echo, redirect, HaltException.
- Reads HOST, PORT, APP_ENV from environment; defaults to 127.0.0.1:3000 / development.
- SIGINT handler calls server.stop() for graceful shutdown.
