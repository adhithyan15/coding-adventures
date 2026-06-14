# Changelog

## 0.1.0 — 2026-04-27

Initial release. Eight-route demo for the Java Conduit framework, mirroring the
Ruby/Python/Lua/TypeScript/Elixir/Rust conduit-hello demos route-for-route.

### Added
- `ConduitHello.app()` — pure factory building the 8-route application.
- `main()` entry point (`gradle run`), with `--host`/`--port` flags.
- 14 integration tests over real HTTP via `java.net.http.HttpClient`.
- Depends on the `conduit` package through a Gradle composite build.
