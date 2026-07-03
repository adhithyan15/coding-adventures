# Changelog — conduit-hello (Dart)

## 0.1.0 — 2026-06-14

Initial release (WEB17).

- Demo program showing idiomatic Dart usage of `coding_adventures_conduit`.
- API-key before-filter, after-hook metadata stamping, route params, query string, echo, redirect, HaltException.
- Reads HOST, PORT, APP_ENV from environment; defaults to 127.0.0.1:3000 / development.
- SIGINT handler calls server.stop() for graceful shutdown.
