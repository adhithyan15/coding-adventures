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
- Reject transfer-encoding/content-length ambiguity, non-final or repeated
  request chunking, conflicting duplicate lengths, whitespace-before-colon,
  invalid field tokens and controls, and variable start-line delimiters.
- Require response request-method context so HEAD is bodyless and successful
  CONNECT responses report their tunnel transition.
- Bound heads to 65,536 bytes, lines to 8,192 bytes, fields to 100, and
  transfer codings to 16 before byte-to-string conversion.
- Keep typed errors redacted so malformed targets and field values cannot leak
  through routine error rendering.
- Reject incomplete heads, malformed start lines, invalid versions, bounded
  status overflow, malformed headers, and bounded content-length failures with
  stable typed errors.
- Add 27 Hspec examples covering the 13 NET04 normative cases plus leading
  blanks, framing precedence, exact offsets, status boundaries, oversized
  decimals, duplicate-length agreement, unsafe framing, strict grammar,
  resource limits, redacted errors, empty reasons, and arbitrary byte input.
- Measure 96% expression, 87% alternative, and 80% top-level-definition
  coverage with GHC 9.4.8.
