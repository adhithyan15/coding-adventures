# Changelog — coding_adventures_zstd

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.3] — 2026-08-03

Repo-wide audit follow-up: a sibling effort fixing `java/zstd` (PR #9780,
rescuing it from a stale branch and running it against the real `zstd` CLI
for the first time) discovered a real RFC 8878 conformance bug in the
sequences-section FSE codec, later confirmed also present in `rust/zstd`.
This release audits and fixes the same bug class here. See lessons.md
Lesson 96 (FSE codec) and Lesson 95 (FHD checksum-flag bit) for full detail.

### Fixed

- **Three compounding bugs in the sequences-section FSE codec**, found by
  cross-checking against the real `zstd` CLI (new TC-9 test below) and the
  reference C source (`fse.h` / `fse_decompress.c` /
  `zstd_decompress_block.c` from github.com/facebook/zstd). All three were
  invisible to every prior test in this package, because encoder and
  decoder were wrong in the same self-consistent way — internal round-trip
  tests can never catch a systematic, symmetric protocol deviation, only an
  independent reference implementation can:
  1. `build_decode_table/2` and `build_encode_sym/2` used a fabricated
     two-pass symbol-table spread ("count > 1 first, then count == 1")
     instead of the real single pass over symbols in ascending index order
     (`FSE_buildDTable_internal`'s actual algorithm). There is no
     correctness reason for the count>1-vs-count==1 split — it was a
     spurious deterministic-looking convention with no basis in the
     reference algorithm.
  2. Per-sequence field order was wrong on both sides. The real decoder:
     PEEKS all 3 FSE symbols (LL, ML, OF) from the current states first
     (a bare table lookup — consumes NO bits), THEN reads extra bits in
     order Offset, Match_Length, Literals_Length, THEN updates FSE states
     (consuming bits) in order Literals_Length, Match_Length, Offset. The
     initial states (read once, before any sequences) are read in order LL,
     OF, ML — a DIFFERENT order from the per-sequence update order; RFC
     8878 is asymmetric here. This package previously fused "peek" and
     "state update" into one step (in the wrong order, LL/OF/ML) and read
     extra bits before doing so, corrupting the bitstream position for
     every stream that followed the first sequence.
  3. The FSE state-transition update must be SKIPPED for the LAST sequence
     in a block — there is no "next" sequence to prepare a state for, and
     symmetrically the encoder must never flush bits for that non-existent
     transition. Added `fse_init_state/3` (mirrors the real zstd reference's
     `FSE_initCState2`), which derives the encoder's starting state directly
     from a symbol via a rounding formula (`(delta_nb + 2^15) >>> 16`)
     instead of the normal `(state + delta_nb) >>> 16` transition, writing
     no bits. `apply_sequences/11` now skips the state-update read on the
     decode side's last iteration (`remaining == 1`) to match.
  - Confirmed this bug class also reproduced against `rust/zstd` with the
    same minimal repro (`compress("ababababab" * 3)`, one sequence:
    ll=2, ml=28, offset=2) before this fix — not an Elixir-specific porting
    mistake. Flagged as a follow-up for the remaining `zstd` ports.
- **Frame Header Descriptor `Content_Checksum_Flag` bit position**: the
  comment above the encoder's fixed FHD byte (`0xE0`) mislabelled bit 4 as
  `Content_Checksum_Flag` (it is actually `Unused_bit`; the real checksum
  flag is bit 2, per RFC 8878 §3.1.1.1 and verified empirically —
  `zstd -c file` emits FHD `0x64`, `zstd -c --no-check file` emits FHD
  `0x60`, differing at bit 2). Since `0xE0` already has both bits clear,
  this was comment-only and never caused a wire-format bug in this package;
  fixed the documentation to prevent the mistake from spreading if
  checksum support is ever added. No functional change.

### Added

- **TC-9 (CLI interoperability)**: two new tests exercise the real `zstd`
  CLI binary via `System.cmd/3`, in both directions (compress here,
  decompress with `zstd -d`; and compress with `zstd`, decompress here),
  gracefully skipped when `zstd` isn't found on `PATH`. This is the test
  that would have caught (and, once written, did catch) the FSE bug above —
  every existing test in this package only ever round-tripped through our
  own encoder/decoder pair. Covers the exact minimal repro from the bug
  report, prose text, and a ~9 KB high-sequence-count input (to also
  exercise the `Number_of_Sequences` 2-byte wire form past the 128-sequence
  boundary).
- The previous "TC-9: bad magic returns error" test is renamed to
  "bad magic returns error" (dropping the TC-9 label) — per
  `code/specs/CMP07-zstd.md`, TC-9 is Cross-language / interoperability;
  the old label collided with the spec's numbering.

### Security

- Hardened the new TC-9 tests' temp-file names (`unique_tmp_name/1`): mixes
  `:crypto.strong_rand_bytes/1` output in alongside the monotonic counter,
  so a co-resident local user can't pre-guess the path and symlink-race it
  between `File.write!/2` and the `zstd` CLI's read. Flagged as LOW severity
  in security review (test-only temp files, no sensitive content); fixed
  anyway since the change was cheap.

## [0.1.2] — 2026-07-12

### Fixed

- `TC-8: 300 KB repetitive text round-trip with compression` now carries
  `@tag timeout: 300_000`. The pure-Elixir compressor makes a single bounded
  pass over the 300 KB buffer, but on a saturated CI runner that could exceed
  ExUnit's 60 s default and turn transient load into a red build.
- Grouped the two `encode_blocks/3` clauses together (the `is_all_same/2`
  helper had been defined between them), clearing the compiler's
  "clauses with the same name and arity should be grouped together" warning.

## [0.1.1] — 2026-04-26

### Tests

- Added `seq_count: 200 KB repetitive text — endianness regression`. The
  test round-trips 200 KB of repetitive ASCII, which reliably yields ≥ 128
  sequences in a single block — exercising the 2-byte path of
  `encode_seq_count` / `decode_seq_count`. Same shape as the regression
  added to TS+Go in PR #1448.
- Audited `encode_seq_count` / `decode_seq_count`: already RFC 8878
  §3.1.1.3.1-compliant (`(cnt >>> 8) ||| 0x80, cnt &&& 0xFF`); no fix needed.

## [0.1.0] — 2026-04-24

### Added

- **`CodingAdventures.Zstd.compress/1`** — Compress any binary to a valid ZStd
  frame (RFC 8878). Supports Raw, RLE, and Compressed block types. Falls back
  to Raw when LZ77 + FSE does not reduce size.

- **`CodingAdventures.Zstd.decompress/1`** — Decompress a ZStd frame, returning
  `{:ok, binary}` or `{:error, reason}`. Supports frames produced by our encoder
  as well as handcrafted Raw-block frames. Guards against decompression bombs
  (256 MB output cap).

- **FSE (Finite State Entropy) codec** — Full encoder and decoder using the
  predefined distributions from RFC 8878 Appendix B. No per-frame table
  transmission needed. Both encoder and decoder share the same spread function
  (step = (sz >> 1) + (sz >> 3) + 3) for exact symmetry.

- **RevBitWriter / RevBitReader** — Reverse bit-stream codec for the FSE sequence
  bitstream. The writer prepends bytes (then reverses at flush) and attaches a
  sentinel bit so the reader can locate the start without side-channel data.

- **Multi-block support** — Inputs larger than 128 KB are split into multiple
  blocks. Back-references correctly span block boundaries (the output accumulator
  is threaded through all block decompressors).

- **Sequence count encoding fix** — Follows RFC 8878 §3.1.1.1.3: 2-byte counts
  use `byte0 = (count >> 8) | 0x80` (high byte first, bit-7 set) rather than
  a raw little-endian u16. This ensures the decoder can distinguish 1-byte and
  2-byte encodings by inspecting byte0 alone.

- **24 unit + integration tests** — TC-1 through TC-9 from the spec, plus
  additional round-trip, compression-ratio, wire-format, and edge-case tests.
  Coverage: 90% (threshold: 80%).

- **Literate inline documentation** — All internals explained with diagrams,
  bit-layout examples, and algorithm justifications for each phase of the FSE
  table construction.

### Dependencies

- `coding_adventures_lzss` (local path `../lzss`) — provides LZ77 tokenisation.
