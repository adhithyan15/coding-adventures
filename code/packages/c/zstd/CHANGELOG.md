# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-08-05

### Added

- Pure ISO C17 port of ZStandard (CMP07, RFC 8878 educational subset):
  raw literals, predefined-only FSE sequence coding (RFC 8878 Appendix B,
  accuracy logs 6/6/5), no repeat-offset shortcuts, no dictionary support,
  Raw/RLE/Compressed block types, Single_Segment frames with an 8-byte
  Frame_Content_Size, 128 KB block cap.
- API: `zstd_compress` / `zstd_decompress` (`uint8_t**` malloc'd out-params,
  `ZstdStatus` return: `ZSTD_OK` / `ZSTD_ERR_ALLOC` / `ZSTD_ERR_FORMAT`),
  mirroring `c/lzss`'s status-enum style. Depends on `c/lzss` (CMP02) for the
  LZ77 match-finder used to build sequences.
- Ported the FSE sequences-section codec from the CORRECTED
  `code/packages/rust/zstd` reference (validated against the real `zstd`
  CLI on 2026-08-03/04) — not from a fresh re-derivation of RFC 8878 text.
  Specifically avoids the bug class every other language's `zstd` port in
  this repo independently reinvented:
  - FSE decode/encode table-spread is a single ascending pass over symbols
    `0..maxSymbolValue`, matching `FSE_buildDTable_internal`'s real
    algorithm — not a fabricated two-pass "count>1 then count==1" split.
  - Per-sequence field order: peek all three symbols (LL, ML, OF) from
    current state first (free, no bits consumed), then read extra bits in
    order OF, ML, LL, then update states in order LL, ML, OF.
  - The state-transition update is skipped for the last sequence in a
    block; the encoder's reverse loop initialises that sequence's starting
    state directly via `fse_init_state` (mirrors `FSE_initCState2`),
    writing zero bits, rather than flushing a transition no decoder read
    would ever consume.
  - `Number_of_Sequences`'s 2-byte wire form writes the marker+high byte
    FIRST, not as a plain little-endian `count | 0x8000` pair — the
    boundary this fixes is exercised by a dedicated
    high-sequence-count interop regression test (9000-byte periodic input,
    single block, >128 sequences).
  - Frame Header Descriptor `Content_Checksum_Flag` is bit 2, not bit 4
    (Lesson 95) — verified against real `zstd -c` / `zstd -c --no-check`
    output.
- Full Repeated_Offset (R1/R2/R3) DECODE support (RFC 8878 §3.1.1.3.2.1.1,
  including the "Literals_Length == 0 shifts the repeat-offset interpretation
  by 1" special case), threaded as frame-scoped state (default 1/4/8,
  persisting across Raw/RLE blocks, updated after every Compressed block's
  sequences) through `zstd_decompress`. This is a genuine gap the Rust
  reference (and, by inheritance, this port's first draft) shared: the
  ENCODER intentionally never emits repeat-offset codes (an explicit
  educational-subset simplification — see zstd.h), so a self-consistent
  round trip never exercises repeat-offset decoding, and neither does the
  spec's single fixed TC-9 corpus. Found by fuzzing this port against the
  real `zstd` CLI across varied inputs (random/periodic/constant/ramp byte
  patterns) beyond the committed test suite before pushing — the real CLI's
  encoder uses repeat offsets constantly, so a decoder that only understood
  explicit offset codes would silently fail to decode a large fraction of
  real-world `.zst` files. See lessons.md Lesson 98 for the full writeup and
  the reference-C-source cross-check (`ZSTD_decodeSequence`).
- Security hardening beyond the Rust reference's current state, per the
  spec's explicit security-considerations checklist: `zstd_decompress`
  rejects any block claiming `Block_Size > 1 << 17` (128 KB) before trusting
  it for bounds arithmetic. Decompression-bomb protection (256 MB output
  cap, checked incrementally per literal run and per match copy, never via
  Frame_Content_Size pre-allocation), offset-bounds checks on every
  back-reference, and Predefined-only Symbol Compression Mode enforcement
  all carried over from the reference design.
- Tests (`tests/zstd_test.c`, via the shared `iso-harness`): all 10 mandatory
  conformance cases from `code/specs/CMP07-zstd.md` (TC-1..TC-10), including
  TC-9's real `zstd` CLI interoperability test in BOTH directions (compress
  here / decompress with the real CLI, and vice versa) via `system()` +
  temp files — pure ISO C, no POSIX-only `popen`/`fork`, so it compiles
  unmodified under MSVC. Degrades to a skip notice (not a failure) when no
  `zstd` binary is on `PATH`. Plus an extra high-sequence-count CLI interop
  regression test, and malformed/adversarial-input safety checks (truncated
  frame, bad magic, oversized block-size claim, reserved block type).
