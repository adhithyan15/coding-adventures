# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-05-05

### Added

- Rust-native `Application` facade over `web-core::WebApp`.
- Route registration helpers for `GET`, `POST`, `PUT`, `DELETE`, `PATCH`, and
  arbitrary HTTP methods.
- Application settings and route introspection.
- Before filters, observing after filters, response-transforming after hooks,
  custom not-found handlers, custom method-not-allowed handlers, and panic
  recovery hooks.
- `Server` wrapper that binds to the native platform backend through
  `web-core::WebServer`.
- Response helpers: `text`, `html`, `json`, `redirect`, `halt`, and
  explicit-status variants.
- `RequestExt` helpers for route params, query params, and body text.
- Tests for routing, hooks, response helpers, request helpers, JSON escaping, and
  real TCP serving on a platform-native backend.
