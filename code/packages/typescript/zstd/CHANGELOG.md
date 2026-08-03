# Changelog — @coding-adventures/zstd

All notable changes to this package will be documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.2] — 2026-08-03

### Fixed

- **FSE sequences-section codec had three compounding conformance bugs** —
  none catchable by internal round-trip tests, all found via real `zstd`
  CLI interop (a sibling audit of `java/zstd` and `rust/zstd` surfaced the
  same bug class; this package had it too). See lessons.md Lesson 96 for
  the full writeup.
  1. `buildDecodeTable` / `buildEncodeTable` spread FSE table symbols using a
     fabricated two-pass split ("all count>1 symbols first, then all
     count==1 symbols", both in ascending symbol order) instead of the real
     algorithm: a SINGLE pass over symbols `0..maxSymbolValue`, placing each
     symbol's full count immediately when encountered
     (`FSE_buildDTable_internal`'s low-probability branch in the reference C
     source). The two-pass version produced a different but internally
     self-consistent table layout.
  2. Per-sequence field order was wrong in two ways. A conformant decoder
     PEEKS all three symbols (LL, OF, ML) from the current states first —
     a bare table lookup that consumes no bits — THEN reads the value extra
     bits in order **OF, ML, LL**, THEN updates the states in order
     **LL, ML, OF**. The previous code combined peek-and-update into one
     step per field and read/wrote the extras and state-updates in the
     wrong relative order (and the wrong OF/ML sub-order).
  3. The state-transition update is skipped entirely after the **last**
     sequence in a block (there is no "next" sequence to prepare a state
     for). The encoder's mirror-image first iteration (its reverse loop
     starts at the semantically last sequence) must derive its starting
     state directly via a new `fseInitState` helper (mirroring real zstd's
     `FSE_initCState2`, which writes no bits) instead of a normal
     bit-flushing `fseEncodeSym` transition. The previous encoder always
     flushed a transition for every sequence uniformly, writing bits a real
     decoder would never read and shifting the bit-alignment of everything
     that followed.

  All three bugs were self-cancelling as long as our own encoder and decoder
  used the same (wrong) convention — every one of the pre-existing 36 tests,
  including internal FSE round-trip coverage, passed regardless of which of
  the three bugs were present, because both sides of every comparison were
  wrong identically. Only testing against an independent, spec-conformant
  implementation (the real `zstd` CLI) could catch it.
- **FHD `Content_Checksum_Flag` documented/commented as bit 4; corrected to
  bit 2** per RFC 8878 §3.1.1.1 (verified empirically: `zstd -c` emits FHD
  byte `0x64`, `zstd -c --no-check` emits `0x60` — the differing bit is
  bit 2). This package always emits `0` for both bits (it never produces a
  content checksum), so the mislabeling was never a functional bug here —
  only a stale/incorrect comment — but it's corrected for accuracy. The
  shared spec (`code/specs/CMP07-zstd.md`) had the same mislabeling and is
  also corrected. See lessons.md Lesson 95.

### Added

- **Real TC-9 CLI-interop test**, previously missing: the package's
  `describe("TC-9: ...")` block actually covered bad-magic error handling,
  not the spec's TC-9 ("Cross-language / interoperability" — compress with
  the real `zstd` CLI and decompress with ours, and vice versa). That block
  is renamed to `"bad magic throws"` and a new
  `"TC-9: cross-language / interoperability (real zstd CLI)"` block is
  added, shelling out to the real `zstd` binary via `node:child_process` in
  both directions, plus a regression case pushing a single block's sequence
  count past 128 (the 1-byte/2-byte wire-format boundary). Both tests are
  skipped gracefully (not failed) when `zstd` isn't on `PATH`.

## [0.1.1] — 2026-04-26

### Fixed

- **`encodeSeqCount` / `decodeSeqCount` now use RFC 8878 §3.1.1.3.1 layout.**
  The 2-byte form previously wrote bytes in the wrong order
  (`[count & 0xFF, (count >> 8) | 0x80]`), placing the LOW byte first. The
  decoder reads byte0 to determine the form: for any count ≥ 128 whose low
  byte was < 128 (e.g. count=515 → byte0=0x03), the decoder mis-took the
  1-byte path and returned a tiny garbage count, mis-aligning every byte
  downstream — including the symbol-modes byte, which then often parsed
  with `LL_Mode != 0` and threw "unsupported FSE modes". Roughly half of all
  counts in the 2-byte range silently corrupted; the other half worked, so
  most existing tests passed.
- New regression test in TC-8 round-trips 200 KB of repetitive text, which
  reliably produces > 128 sequences in a single block.

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of ZStd (RFC 8878) compression and decompression in TypeScript.
- `compress(data: Uint8Array): Uint8Array` — encodes a ZStd frame with:
  - 4-byte magic number `0xFD2FB528`
  - Frame Header Descriptor with 8-byte Frame Content Size
  - Multi-block splitting at 128 KB boundaries
  - RLE block detection (all bytes identical)
  - Compressed blocks via LZ77 (LZSS) + FSE sequence encoding
  - Raw block fallback when compression is not beneficial
- `decompress(data: Uint8Array): Uint8Array` — decodes a ZStd frame with:
  - Magic number validation
  - Full Frame Header Descriptor parsing (FCS, Single_Segment, Dict_ID flags)
  - Raw, RLE, and Compressed block support
  - Predefined FSE mode decoding (LL, ML, OF tables per RFC 8878 Appendix B)
  - 256 MB output-size guard against decompression bombs
- `RevBitWriter` — backward bit accumulator using BigInt register (64-bit safe)
- `RevBitReader` — backward bit reader with sentinel detection
- FSE decode table builder (`buildDecodeTable`) following the ZStd spreading algorithm
- FSE encode table builder (`buildEncodeTable`) with symmetric state transitions
- Predefined distributions: `LL_NORM`/`LL_ACC_LOG`, `ML_NORM`/`ML_ACC_LOG`, `OF_NORM`/`OF_ACC_LOG`
- LL/ML code tables from RFC 8878 §3.1.1.3 (36 and 53 entries respectively)
- Raw literals section encoding/decoding (1-byte, 2-byte, 3-byte headers)
- Sequence count encoding/decoding (1-byte, 2-byte, 3-byte formats)
- Comprehensive test suite (TC-1 through TC-9 plus additional round-trip and unit tests)
- Literate programming style comments throughout — explanations, diagrams, examples
