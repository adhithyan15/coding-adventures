# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `cfb-writer` crate: a from-scratch,
  zero-dependency writer for the OLE2 / Compound File Binary Format (MS-CFB),
  the container inside legacy `.xls` / `.doc` / `.ppt` files.
- `CfbWriter` (opaque) with `cfb_writer_new` / `cfb_writer_free` /
  `cfb_writer_add_stream` / `cfb_writer_finish`, plus the one-shot `cfb_write`.
- Emits deterministic version-3 files: 512-byte sectors, the FAT (with a
  fixed-point sector-count loop so the FAT describes its own sectors), a
  directory of 128-byte entries (all-black tree, insertion-order sibling chain),
  and a mini-stream + mini-FAT for streams under the 4096-byte cutoff.
- UTF-8 stream names are transcoded to on-disk UTF-16LE and truncated to 31 code
  units. Every allocation is checked and `size_t`-overflow-guarded; the build
  unwinds to NULL on failure.
- 270 checks: header/structure assertions plus a round-trip through an in-test
  CFB reader (empty / mini / large / multi-FAT-sector / multi-sector-mini-stream
  streams, name truncation, UTF-8→UTF-16 encoding, determinism), run under every
  ISO C compiler via the shared `iso-harness`; also clean under ASan + UBSan.
