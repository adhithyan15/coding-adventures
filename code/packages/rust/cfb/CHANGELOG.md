# Changelog

All notable changes to the `cfb` crate are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this
project adheres to Semantic Versioning.

## [0.1.0] — 2026-07-03

### Added

- Initial release: a from-scratch, zero-dependency reader for the OLE2 /
  Compound File Binary Format ([MS-CFB]) — the container inside legacy
  `.xls` / `.doc` / `.ppt` files. `#![forbid(unsafe_code)]`.
- **Header parsing**: signature check, sector shift (512- and 4096-byte
  sectors), mini-sector shift, FAT/mini-FAT/DIFAT/directory locations,
  mini-stream cutoff.
- **FAT assembly** from the header-inlined DIFAT (first 109 entries) plus the
  DIFAT-sector chain for larger files.
- **Directory** parsing (128-byte entries, UTF-16LE names with raw control-char
  prefixes preserved) and recursive red-black-tree flattening into a linear
  entry list.
- **Both storage paths**: streams ≥ cutoff read from the regular FAT + sectors;
  streams < cutoff read from the mini-stream via the mini-FAT.
- Public API: `CompoundFile::open`, `entries`, `stream_names`, `read_stream`
  (case-insensitive top-level lookup), `read_stream_by_id`.
- `CfbError` taxonomy: `BadSignature`, `Truncated`, `UnsupportedSectorSize`,
  `BadSectorChain`, `CycleDetected`, `OutputTooLarge`, `BadDirectory`,
  `NotAStream`.
- **Security hardening** for untrusted input: cycle-guarded chain walks
  (visited-set + hard iteration cap), bounds-checked sector offsets with
  checked arithmetic, and a 256 MiB total-output cap. No `unwrap`/`panic!` on
  input bytes.
- 20 unit/integration tests + 1 doctest, including an end-to-end read of a real
  minimal `.xls` fixture (asserts the `Workbook` stream is 4096 bytes and starts
  with a BIFF8 BOF record `0x0809`), a crafted mini-stream round-trip, and
  cycle-injection tests proving the reader does not hang.
- Spec: `code/specs/CFB01-compound-file.md` — literate walkthrough of the
  sectors / FAT / directory / mini-stream model.
