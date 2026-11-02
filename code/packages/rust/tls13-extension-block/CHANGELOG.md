# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-29

### Added

- Exact decoding of one two-byte-length-prefixed TLS extension vector.
- Caller-owned payload-byte and entry-count bounds with fixed capacity for 64
  borrowed entries.
- Wire-order retention of registered or unknown extension types and opaque data
  without embedding an extension registry.
- Fail-closed rejection of truncated headers/data, trailing bytes, duplicate
  extension types, and byte/count limit exhaustion.
- Redacted diagnostics and stable payload-free error categories.
