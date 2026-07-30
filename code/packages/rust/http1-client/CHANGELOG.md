# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-29

### Added

- Added bounded synchronous HTTP/1.0 GET requests over `tcp-client`.
- Added response framing through the shared `http1` and `http-core` contracts.
- Added relative 301/302 redirect following with a configurable limit.
- Added response head and body size limits plus request-line/header injection
  guards.
- Added localhost acceptance coverage for content-length, EOF, redirects,
  malformed responses, and unsupported framing.
