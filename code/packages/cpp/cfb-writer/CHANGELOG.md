# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Fixed

- Test bug, not a library bug: the mini-stream case built its input as
  `vector(lit, lit + 17)` from two separate spellings of the same string
  literal, relying on the compiler pooling them into one object. GCC and Clang do;
  MSVC at `/Od` (the iso-harness default) does not, so `last - first` was
  garbage and the constructor threw `length_error`, an uncaught exception that
  exits `0xC0000409`. The test now uses one named array. Reproduced with real
  `cl.exe` (`/W4 /WX`, also under `/fsanitize=address`), confirmed by `/GF`
  (string pooling) making the old test pass, and verified on `cl` and `clang-cl`.

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
