# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Fixed

- Fixed the optimized-IR gate (`tests/optimized_ir.rs`) on Rust/Cargo 1.100:
  Cargo's new build-dir layout emits `--emit=llvm-ir` output under
  `release/build/<crate>/<hash>/out/` instead of `release/deps/`, so the gate
  failed with "release dependency directory must exist" before inspecting any
  IR. The gate now locates the `.ll` file by searching the release tree. No
  assertion was changed or weakened; the IR shape checks pass unmodified on
  both 1.97 and 1.100.

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
