# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-29

### Added

- Bounded borrowed decoding for one already-delimited TLS 1.3 `ServerHello`
  body.
- Fixed legacy-version and compression validation, zero-to-32-byte session-id
  echo framing, and exact unclassified cipher-suite retention.
- Composition with the repository-owned extension-block decoder under
  caller-owned body, extension-payload, and extension-count ceilings.
- Stable payload-free errors and metadata-only diagnostics with no external
  effects.
