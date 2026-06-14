# Changelog

## 0.1.0 — 2026-06-14

Initial release. Kotlin port (WEB10) of the Conduit web framework — a thin
idiomatic DSL over the Java package, reusing the WEB09 `conduit_jni` cdylib
with no new native code.

### Added
- `conduit { }` builder DSL with trailing-lambda routes and filters.
- Top-level response helpers (`html`, `json`, `text`, `respond`, `halt`,
  `redirect`) and `Request` extension properties (`req.path`, `req["name"]`, …).
- `ConduitApp` lifecycle wrapper (`serve`/`serveBackground`/`stop`/`localPort`/
  `running`, `AutoCloseable`).
- Kotlin/JUnit5 tests incl. an end-to-end HTTP suite (class-level 30s timeout).
- Depends on `com.codingadventures:conduit` via a Gradle composite build.
