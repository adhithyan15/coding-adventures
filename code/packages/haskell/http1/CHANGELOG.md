# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-30

### Added

- Add strict-byte parsing for complete HTTP/1 request and response heads.
- Produce Haskell `http-core` request/response values, exact body offsets, and
  `BodyKind` framing without reading payload bytes or performing I/O.
- Accept CRLF and bare-LF heads, skip leading blank lines, preserve duplicate
  header order and colons in values, and trim HTTP optional whitespace.
- Apply request and response body-framing precedence for chunked transfer
  encoding, content length, bodyless response statuses, and EOF-delimited
  responses.
- Reject incomplete heads, malformed start lines, invalid versions, bounded
  status overflow, malformed headers, and bounded content-length failures with
  stable typed errors.
- Add 22 Hspec examples covering the 13 NET04 normative cases plus leading
  blanks, framing precedence, exact offsets, status boundaries, oversized
  decimals, first-length semantics, empty reasons, and arbitrary byte input.
- Measure 98% expression and 96% alternative coverage with GHC 9.4.8.
