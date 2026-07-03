# Changelog

All notable changes to `cfb-writer` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
[Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-07-03

### Added

- Initial release (**CFBW01**): a from-scratch, zero-third-party-dependency
  **writer** for the OLE2 / Compound File Binary Format ([MS-CFB]) — the exact
  inverse of the `cfb` reader crate.
- Public API:
  - `CfbWriter::new()`, `add_stream(name, data)`, `finish() -> Vec<u8>`.
  - `write_cfb(&[(&str, &[u8])]) -> Vec<u8>` convenience function.
- Emits **version 3** files: 512-byte sectors, 64-byte mini-sectors, 4096-byte
  mini-stream cutoff.
- **Both storage paths** implemented:
  - Large streams (≥ 4096 bytes) get their own regular 512-byte FAT sectors.
  - Small streams (< 4096 bytes) are packed into the Root Entry's mini-stream
    and chained by the mini-FAT.
- **Fixed-point FAT-sector count**: the FAT describes its own sectors, so the
  sector count is iterated to a fixed point.
- Directory emitted as a trivially-valid all-black red-black tree; streams are
  chained as right-siblings in insertion order.
- Header + inlined DIFAT (first 109 FAT-sector locations) written per spec.
- **Determinism**: CLSID and all timestamp fields zeroed; identical input
  yields identical bytes.
- **Robustness**: `#![forbid(unsafe_code)]`, no `unwrap`/`expect`/`panic!` on
  the public path; overlong names truncated to 31 UTF-16 units; empty streams
  and an empty stream set both produce valid files; overflow-safe sector-count
  arithmetic.
- **Round-trip proof**: writes mixed small + large streams and reopens them with
  the `cfb` reader, asserting byte-for-byte equality. 22 unit tests + 2
  doctests, all passing; clippy clean under `-D warnings`.

### Known limitations

- Storages (nested folders) are not yet emitted — output is a flat set of
  top-level streams, sufficient for the legacy single-workbook/document case.
- Files needing more than 109 FAT sectors (a DIFAT-sector chain) are out of
  scope for 0.1.0; the safety cap keeps realistic inputs well under that.

[MS-CFB]: https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-cfb/
