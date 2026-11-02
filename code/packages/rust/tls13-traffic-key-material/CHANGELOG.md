# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-29

### Added

- Cipher-suite-bound derivation of the RFC 8446 `"key"` and `"iv"` outputs
  from an already-derived traffic secret.
- Exact SHA-256/SHA-384 matching between the policy-selected suite and the
  fixed-memory HKDF owner before secret derivation begins.
- Non-cloneable caller-owned fixed wipe-on-drop storage plus a success-only
  borrowed key-material view, keeping secrets out of ABI return slots while
  retaining redacted diagnostics and stable payload-free failures.
- Release-codegen regression coverage for successful key/IV staging copies,
  subsequent source-slot scrubs, and allocator absence.
- RFC 8448 SHA-256 evidence plus independent SHA-256 and SHA-384 vectors for
  all three repository-targeted TLS 1.3 cipher suites.
