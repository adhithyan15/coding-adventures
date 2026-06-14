# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- Added `RequestTarget`, raw query iteration, `RequestHead` path/query
  helpers, and `RoutePattern::match_target` so local API dispatchers can
  match routes by path while preserving query strings separately.

## [0.1.0] - 2026-04-18

### Added

- Implemented shared `Header`, `HttpVersion`, `BodyKind`, `RequestHead`, and `ResponseHead` types
- Added helper functions for case-insensitive header lookup plus `Content-Length` and `Content-Type`
- Added unit tests covering version parsing, header lookup, content helpers, and semantic head helpers
