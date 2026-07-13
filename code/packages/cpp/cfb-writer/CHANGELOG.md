# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `cfb-writer` crate in namespace
  `ca::cfb_writer`: a zero-dependency writer for the OLE2 / Compound File Binary
  Format (MS-CFB).
- `CfbWriter` (`add_stream` / `finish`) plus the one-shot `write_cfb`. Emits
  deterministic version-3 files: FAT (fixed-point sector count), a 128-byte-entry
  directory (all-black tree, insertion-order sibling chain), and a mini-stream +
  mini-FAT for streams under the 4096-byte cutoff.
- UTF-8 names transcoded to UTF-16LE, truncated to 31 code units. Ownership is
  automatic via `std::vector` / `std::string`.
- 94 checks: header/structure assertions plus a round-trip through an in-test
  CFB reader, run under every ISO C++ compiler via the shared `iso-harness`;
  also clean under ASan + UBSan.
