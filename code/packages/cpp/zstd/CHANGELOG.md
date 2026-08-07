# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-08-05

### Added

- Pure ISO C++17, header-only implementation of an educational RFC 8878
  subset of ZStandard (CMP07), in namespace `ca::zstd`: raw literals only,
  predefined-mode FSE tables only for the sequences section (RFC 8878
  Appendix B `LL_NORM`/`ML_NORM`/`OF_NORM`, accuracy logs 6/6/5), no
  repeat-offset (R1/R2/R3) shortcuts, no dictionary support, no checksum by
  default, frames always Single_Segment with an 8-byte FCS, blocks capped at
  128 KB.
- Reuses `cpp/lzss` (CMP02) as the LZ77 back-reference tokenizer (window
  32 KB, max match 255, min match 3 — bounded to fit `lzss::Token`'s
  `uint16_t offset` / `uint8_t length` fields).
- API: one-shot `compress(data)` (never throws — every input has a valid
  encoding, worst case a Raw block copy) and `decompress(data)` (throws
  `zstd::ZstdError`, matching `cpp/wasm-leb128`'s
  `Error : public std::runtime_error` convention, since a ZStd frame carries
  untrusted, security-relevant framing that must be validated rather than
  silently clamped).
- Implemented directly against the ALREADY-CORRECTED `code/packages/rust/zstd`
  reference (post repo-wide FSE-codec audit), not re-derived from scratch —
  avoiding the same bug class independently reinvented by every
  pre-existing language port. See `include/zstd.hpp`'s "THE FSE BUG CLASS"
  banner comment for the three specific rules this implementation follows:
  (1) FSE table-spread is a single pass over symbols `0..maxSymbol`, never a
  fabricated two-pass "count>1 then count==1" split; (2) per-sequence decode
  order is peek all three symbols (LL/ML/OF, free) → read extra bits in
  order OF/ML/LL → update states in order LL/ML/OF, with the ONE-TIME
  initial-state read (before sequence 1) in the DIFFERENT order LL/OF/ML;
  (3) the state-transition update is skipped entirely for the last sequence
  in a block, and the encoder's mirror-image first-processed symbol gets its
  starting state via a direct `FSE_initCState2`-style formula (zero bits
  written), not a normal bit-flushing transition.
- Frame Header Descriptor `Content_Checksum_Flag` correctly read from bit 2
  (not bit 4 — Lesson 95).
- `Number_of_Sequences`'s 2-byte wire form correctly writes the marker/high
  byte FIRST (`byte0 = (count>>8)|0x80, byte1 = count&0xFF`), not a plain
  little-endian pair.
- Decoder rejects trailing bytes after the frame's end (past an optional
  4-byte content checksum, which is skipped-but-not-verified since this
  package has no xxHash64 implementation) — Lesson 94: a strict decoder must
  surface truncation/concatenation, not silently ignore what follows a
  valid-looking frame. This is enforced even though the upstream Rust
  reference this package translates from does not yet enforce it (see the
  reference's own decoder for context); both the spec and Lesson 94 require
  it, and it does not affect interop with the real `zstd` CLI's own output.
- Security hardening per `code/specs/CMP07-zstd.md`: `Frame_Content_Size` is
  read but never used to pre-allocate output; a 256 MB decompressed-output
  budget is checked incrementally at every point output can grow (including
  inside the per-sequence loop of Compressed-block decoding, not just once
  per top-level Raw/RLE block); block sizes are capped at 128 KB on decode;
  match offsets are bounds-checked before copying (`offset == 0` or
  `offset > bytes_decoded_so_far` both rejected); `Sequences_Count` is
  validated against the 24-bit wire field's limit before use; only
  `Predefined_Mode` (all-zero Symbol_Compression_Modes byte) is accepted for
  LL/OF/ML, rejecting custom/RLE/repeat FSE modes this subset doesn't
  implement.
- Tests (`tests/zstd_test.cpp`) cover all 10 mandatory cases from
  `code/specs/CMP07-zstd.md`: TC-1..TC-8 round trips and compression-ratio
  assertions, **TC-9 real `zstd` CLI interoperability in both directions**
  (compress-here/decompress-there and the reverse, plus a high-sequence-count
  regression probing the `Number_of_Sequences` 1-byte/2-byte wire-encoding
  boundary — the class of bug that a same-codebase round-trip test can never
  catch; gracefully skips if `zstd` isn't on `PATH`), and TC-10 a hand-built
  minimal wire-format frame decoded independently of this package's own
  encoder. Also: malformed-input safety checks (bad magic, truncation,
  trailing bytes, truncated literals section) that must throw rather than
  crash, and internal FSE-codec unit tests pinning the exact
  peek/extras-order/update-order contract.
- CLI interop is implemented via `std::system()` (standard ISO C++,
  `<cstdlib>`) with shell redirection to temp files under `_build/` — pure
  ISO C++17 has no process-spawning API, so this is the same approach the
  `iso-harness`'s own build scripts use, just invoked from C++ rather than
  shell.
