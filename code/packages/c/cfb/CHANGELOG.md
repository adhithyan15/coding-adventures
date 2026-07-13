# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `cfb` crate: a reader for the OLE2 / Compound
  File Binary Format ([MS-CFB]) — the container inside legacy `.xls`/`.doc`/
  `.ppt` files. The read counterpart to the ported `cfb-writer`.
- `cfb_open` (owned `CompoundFile`) / `cfb_free`; header validation; DIFAT/FAT/
  mini-FAT assembly; directory parsing and red-black-tree flattening (iterative,
  visited-guarded); `cfb_sector_size`, `cfb_entry_count`/`cfb_entry`,
  `cfb_read_stream` (by name, ASCII case-insensitive) and
  `cfb_read_stream_by_id`.
- Hostile-input hardening faithful to the Rust: every sector-chain walk is
  cycle-guarded by a step cap (a valid chain can never exceed the FAT slot
  count), the directory walk uses a visited bool array, every offset is
  bounds-checked with overflow-safe arithmetic, and output is capped at 256 MiB.
  Growable buffers guard `size_t` overflow. Verified clean under ASan + UBSan,
  the macOS `leaks` tool (0 leaks), a truncation fuzz over every prefix, and a
  200k-iteration random byte-flip fuzz.
- Documented divergences: names decode UTF-16 LE → UTF-8 into fixed 128-byte
  buffers; `CfbError` drops the sector size the Rust variant carries;
  case-insensitive matching is ASCII-only.
- 26 checks run under every ISO C compiler via the shared `iso-harness`,
  exercising the full read path against crafted in-memory CFB files: mini-stream
  round-trip, multi-entry flatten, directory-tree and FAT-chain cycle detection,
  and error paths.
