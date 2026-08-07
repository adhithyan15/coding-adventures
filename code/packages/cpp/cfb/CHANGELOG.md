# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `cfb` crate in namespace
  `ca::cfb`: a reader for the OLE2 / Compound File Binary Format ([MS-CFB]). The
  read counterpart to the ported `cfb-writer`.
- `CompoundFile::open` (throws `CfbError` where the Rust `open` returns
  `Result`), `entries()`, `stream_names()`, `sector_size()`, `read_stream`
  returning `std::optional<std::vector<std::uint8_t>>` (ASCII case-insensitive),
  and `read_stream_by_id` (throws). RAII over `std::vector` throughout.
- Faithful hostile-input hardening: cycle-guarded sector-chain walks (step cap),
  a `std::vector<bool>` visited guard for the directory walk, overflow-safe
  bounds checks, and a 256 MiB output cap. Verified clean under ASan + UBSan and
  a truncation fuzz over every prefix of a valid file.
- 21 checks run under every ISO C++ compiler via the shared `iso-harness`,
  exercising the full read path against crafted in-memory CFB files (mini-stream
  round-trip, multi-entry flatten, directory-tree and FAT-chain cycle detection,
  error paths).
