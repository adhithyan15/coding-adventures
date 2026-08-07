# Changelog

## [0.1.3] - 2026-08-05

### Fixed

- **`decompress` now implements Repeated-Offset (R1/R2/R3) sequence decoding**
  (RFC 8878 §3.1.1.3.2.1.1). Previously every decoded `Offset_Value` was
  treated as an explicit offset (`offset = of_raw - 3` unconditionally), which
  underflows for `of_raw` in `{1, 2, 3}` — the reserved range that instead
  means "reuse one of the three most-recently-used match offsets" (default
  `1/4/8`, threaded frame-scoped through every Compressed block). The old code
  rejected these as `"decoded offset underflow"`, even though the frame was
  perfectly valid. This package's own `compress()` never emits offset codes
  below 2 (an intentional encoder-side simplification — the minimum LZ77
  match offset here is 1, so `raw_off = offset + 3 >= 4` always), so no
  internal round-trip test, and not even the existing pangram-x25 Spec TC-9
  corpus, ever exercised this decode path. But the real `zstd` CLI's encoder
  uses repeat offsets constantly — they're one of its principal entropy wins,
  especially for periodic or highly repetitive input — so any decoder that
  only understood explicit offsets would systematically fail to decode a
  meaningful fraction of real-world `.zst` files. Confirmed with a direct
  repro: 4713 bytes of a single repeated byte, compressed with the real
  `zstd` CLI, produces one Compressed block with a single sequence at
  `Offset_Value = 1` ("reuse Repeated_Offset1"), which the old decoder
  rejected outright.

  Found via an independent audit after the sibling `c/zstd` port (PR #9941)
  hit and fixed the identical gap while fuzzing itself against the real
  `zstd` CLI — see lessons.md Lesson 98. The fix here was cross-checked
  against both that PR's verified reference implementation and the RFC 8878
  prose directly (fetched live), including the "when `Literals_Length == 0`,
  the repeat-offset codes shift by one, and `Offset_Value == 3` means
  `Repeated_Offset1 - 1`" special case, and the exact history-rotation rule
  (no rotation on R1 reuse; swap-to-front on R2; full rotate on R3 / the
  `LL==0` "R1-1" case). The three offset registers are now frame-scoped (not
  block-scoped): `M.decompress` owns a `{rep1, rep2, rep3}` table, initialized
  to `{1, 4, 8}` per RFC 8878, and threads it (mutated in place) through every
  Compressed block's `decompress_block` call for the rest of the frame;
  Raw/RLE blocks don't touch it. `compress()` is intentionally unchanged —
  this is a decode-only fix, since the encoder-side "no repeat-offset
  shortcuts" simplification remains valid and doesn't affect interop with
  real `zstd -d`.

### Added

- Two new Spec TC-9 tests: decoding real `zstd`-CLI-produced output that uses
  a Repeated-Offset R1 sequence (a long constant-byte run — the exact input
  that surfaced the bug), and output that rotates through all three offset
  registers (several distinct repeated-content regions at different
  distances back to back). Both skip gracefully when `zstd` isn't on `PATH`.
  Also verified ad hoc (outside the committed suite) with a 180-trial fuzz
  sweep — random, periodic, constant-run, and ramp byte patterns across nine
  sizes from 1 byte to 5000 bytes — compressed by the real `zstd` CLI and
  decoded by this package, all byte-exact; and against the existing fixed
  TC-1..TC-11 and Spec TC-9 suite (all 21 checks, unaffected, since this
  package's own round trip never touches the new code path).

## [0.1.2] - 2026-08-03

### Fixed

- **Three compounding RFC 8878 conformance bugs in the sequences-section FSE
  codec**, found via a repo-wide audit after the same bug class was confirmed
  in `java/zstd`, `kotlin/zstd`, and `rust/zstd` (see lessons.md Lesson 96).
  All three bugs were invisible to this package's own round-trip tests
  because encode and decode agreed with each other using the identical wrong
  convention — only decompressing our output with the real `zstd` CLI (new
  Spec TC-9 tests, below) surfaced them:
  1. `build_decode_table`/`build_encode_sym` spread FSE table slots using a
     fabricated two-pass split ("all symbols with count > 1 first, then all
     symbols with count == 1", both in ascending symbol order). The real
     algorithm (`FSE_buildDTable_internal`'s low-probability branch in the
     reference C implementation) is a SINGLE pass over symbols 0..N in
     ascending order, placing each symbol's full count immediately. Fixed to
     a single pass in both functions.
  2. Per-sequence field order was wrong: the decoder now PEEKS all three
     symbols (LL/ML/OF) from the current states first (free — no bits
     consumed), THEN reads extra bits in order **OF, ML, LL**, THEN updates
     FSE states in order **LL, ML, OF** — previously the code combined
     peek-and-update into one step, in the wrong relative order, and read
     extras in the wrong sub-order too. The encoder mirrors this in reverse.
  3. The FSE state-transition **update is now skipped for the last sequence
     in a block** on both sides: the decoder no longer reads update bits
     after decoding the final sequence (there is no "next" sequence to
     prepare a state for), and the encoder's first-processed sequence (the
     reverse loop's first iteration — semantically the LAST real sequence)
     now derives its starting state directly via a new `fse_init_state`
     helper (mirroring real zstd's `FSE_initCState2`, zero bits written)
     instead of a normal bit-flushing transition. Previously both sides
     always performed a transition unconditionally, silently shifting the
     bit-alignment of every sequence that followed.
- **Frame Header Descriptor `Content_Checksum_Flag` is bit 2, not bit 4**
  (RFC 8878 §3.1.1.1.1; bit 4 is `Unused_bit`). Verified empirically against
  the real `zstd` CLI (`zstd -c` emits FHD `0x24`/checksum-on by default;
  `zstd -c --no-check` emits FHD `0x20`/checksum-off — the differing bit is
  bit 2). `decompress()` now reads the correct bit and, when set, skips the
  trailing 4-byte content checksum before the existing trailing-bytes
  rejection check — previously a real checksummed `.zst` frame (the CLI's
  default) would be misparsed as having unexpected trailing data. `compress()`
  still never sets the flag (no xxHash64 implementation here), so this
  package's own output is byte-identical either way; only decoding
  externally-produced checksummed frames was affected. See lessons.md
  Lesson 95.

### Added

- **Spec TC-9 (cross-language interoperability)**: three new tests that shell
  out to the real `zstd` CLI via temp files, covering both directions
  (compress with ours / decompress with `zstd -d`, and compress with `zstd`
  / decompress with ours) — one on a high-sequence-count synthetic corpus
  (~17 KB, thousands of FSE-coded sequences) specifically chosen to exercise
  the last-sequence-skip fix, and one on the spec's own pangram-x25 example
  (matching `java/zstd`'s `tc9CliInterop` test). Skipped, not failed, when
  `zstd` isn't on `PATH`. This is the class of test that actually proves
  wire-format conformance — a same-codebase round-trip test can never catch
  a systematic, symmetric protocol deviation, since both sides of the
  comparison are wrong in the identical way. Temp files use `os.tmpname()`'s
  own atomically-reserved path directly (no derived/concatenated filenames,
  which would reintroduce a symlink race), and shell arguments are quoted
  with a real POSIX single-quote escaper rather than Lua's `%q` (which is
  Lua-source quoting, not shell quoting) — findings from this change's own
  security review.

## [0.1.1] - 2026-04-27

### Fixed

- **`decompress` now rejects trailing bytes after the last block.**  The block
  loop previously broke on `last_block == 1` and returned successfully even if
  bytes remained in the input.  Garbage bytes, truncation artifacts, or a
  concatenated second frame were silently ignored.  A `pos <= #data` check
  after the loop now raises `"unexpected trailing data"`.
- New `TC-11` tests: a valid frame with 3 trailing garbage bytes is rejected;
  the same frame without trailing bytes decompresses cleanly.

## [0.1.0] - 2026-04-25

### Added
- Initial implementation of ZStd (RFC 8878) compression/decompression
- Full FSE (Finite State Entropy) encode/decode with predefined tables
- RevBitWriter/RevBitReader for ZStd's backward bitstream format
- Raw, RLE, and Compressed block types
- 256 MB decompression bomb protection
- 9 test cases covering round-trips, compression ratios, and error handling
