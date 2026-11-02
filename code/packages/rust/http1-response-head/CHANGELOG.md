# Changelog

All notable changes to `http1-response-head` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-29

### Added

- Added allocation-free HTTP/1.0 and HTTP/1.1 response-head decoding under
  explicit complete-head, line-length, and header-count limits.
- Added strict CRLF, status-line, field-name, field-value, and obsolete-folding
  rejection with payload-free failures.
- Added borrowed exact-head, reason-phrase, ordered-header, and body-tail views
  with redacted Debug output.

### Security

- The parser performs no I/O and grants no TLS, HTTP transport, redirect,
  body-framing, credential, provider, audit, trace, or OAuth authority.
- Duplicate-field reconciliation, body framing, transfer decoding, and request
  method semantics remain mandatory later policy gates.
