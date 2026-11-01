# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-29

### Added

- Allocation-free decoding of the one-byte-length-prefixed opaque request
  context and exact remaining TLS extension block.
- Caller-owned request-context, extension-payload, and extension-count bounds.
- Borrowed retention of the exact body, opaque context, and ordered
  duplicate-free registered or unknown extensions.
- Fail-closed framing errors, including rejection of the structurally invalid
  empty extension vector, and diagnostics that contain no context or extension
  payload bytes.
