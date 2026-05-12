# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- Added redacted HTTP/1 request/response head summaries that expose method,
  status, version, counts, body offset, and framing without copying request
  targets, header values, reason text, or body bytes into telemetry.

## [0.1.0] - 2026-04-18

### Added

- Implemented HTTP/1 request and response head parsing on top of `http-core`
- Added body framing detection for fixed-length, chunked, bodyless, and until-EOF responses
- Added tests covering CRLF and LF-only input, duplicate headers, bodyless statuses, and malformed heads
