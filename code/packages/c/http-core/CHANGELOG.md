# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `http-core` crate: shared HTTP message
  shapes and syntax-level helpers.
- Route patterns: `http_route_parse` / `http_route_free`, `http_route_match_path`
  and `http_route_match_target` (path-only matching with named `:param` captures
  returned as an owned `HttpPairs` batch).
- Request targets: `http_parse_request_target` (path/query/fragment),
  `http_query_pairs` and `http_query_value` (raw, undecoded query pairs).
- `HttpVersion` parse / `http_version_to_string` ("HTTP/x.y").
- Headers: `http_find_header` (ASCII case-insensitive), `http_parse_content_length`,
  and `http_parse_content_type` (media type + optional charset, quotes trimmed).
- `HttpRequestHead` / `HttpResponseHead` with delegating helpers, and the
  `HttpBodyKind` enum.
- Rust `Result`/`Option` outcomes become status codes; segment/pair arrays guard
  their growth against `size_t` overflow; pure-ISO string helpers replace POSIX
  `strdup`/`strndup`.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): version parsing,
  case-insensitive header lookup, Content-Length/Content-Type, target splitting,
  query pairs/values, and route matching — mirroring the Rust crate's tests.
