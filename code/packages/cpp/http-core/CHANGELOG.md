# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C++17 header-only port of the Rust `http-core` crate, in
  namespace `ca::http`: shared HTTP message shapes and syntax-level helpers.
- `RoutePattern` (parse + `match_path` / `match_target` with named `:param`
  captures), `RequestTarget` (`parse_request_target`, `query_pairs`,
  `query_value`), `HttpVersion` (`parse` -> `std::optional`, `to_string`).
- `find_header` (ASCII case-insensitive), `parse_content_length`,
  `parse_content_type` (media type + optional charset), the `Header` type, the
  `BodyKind` enum, and `RequestHead` / `ResponseHead` with delegating helpers.
- Value semantics with `std::optional` results; `Result<_, String>` outcomes
  become `std::optional`. Query values are returned undecoded.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): version parsing,
  case-insensitive header lookup, Content-Length/Content-Type, target splitting,
  query pairs/values, and route matching — mirroring the Rust crate's tests.
