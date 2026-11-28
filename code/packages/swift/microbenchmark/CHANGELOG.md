# Changelog

- Declare macOS 10.15.4 as the minimum for this Swift package and both of its
  products, preserving throwing CLI output semantics with FileHandle.

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-25

### Added

- Replay accepts any nonblank, NUL-free workload identifier as producer metadata while the indexed-array command keeps its existing identifier.
- Paired collection with monotonic nanosecond timing, balanced seeded order, and checked signed-64-bit checksum arithmetic.
- Exact analysis, strict JSON replay, indexed-array executable, and multi-limb rank-statistics dependency.
- Reject duplicate object keys before Codable replay, including escaped aliases, with bounded nesting and value counts.

### Fixed

- Enforce the complete optional metadata group, reject explicit-null and
  invalid metadata, preserve empty argv values, and map a present-empty primary
  revision variable to `unknown` while suppressing its fallback.
- Run the Windows Swift availability guard and checks in one batch process so the skip covers the full sequence and failures propagate.
- Keep the reusable command-line helper's environment injectable and empty by default; let the executable select only the two revision keys and report a compile-time OS family label instead of querying the host version string.
