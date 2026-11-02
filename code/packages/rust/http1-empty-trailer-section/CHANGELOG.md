# Changelog

All notable changes to `http1-empty-trailer-section` will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-29

### Added

- Added exact CRLF decoding for an empty trailer section with borrowed encoding
  and remainder retention.
- Added payload-free errors and redacted Debug output.

### Security

- Every trailer field is rejected until its syntax and semantic policy receive
  a separate review.
- Terminal-size lines, aggregate body bounds, transport, TLS, credential,
  audit, trace, and live-provider composition remain absent.
