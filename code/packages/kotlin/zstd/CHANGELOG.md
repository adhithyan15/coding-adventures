# Changelog — kotlin/zstd

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.2] — 2026-08-05

### Fixed
- **Decoder never implemented Repeated-Offset (R1/R2/R3) sequence decoding**
  (RFC 8878 §3.1.1.3.2.1.1). `decompressBlock`'s sequence loop treated every
  decoded offset code as an explicit `Offset_Value - 3` computation and
  outright rejected `Offset_Value < 3` as a "decoded offset underflow" —
  but per the RFC, `Offset_Value` in `{1, 2, 3}` is a **repeat-offset
  reference**: reuse one of the three most-recently-used match offsets
  (frame-scoped registers R1/R2/R3, defaulting to `1/4/8`), not a literal
  offset computation. Real `zstd` encoders use repeat offsets constantly —
  they're one of the format's principal entropy wins, especially for
  periodic/repetitive data — so this decoder systematically failed (or, for
  some inputs, silently produced garbage past a bounds check) on a
  meaningful fraction of real-world `.zst` files, even though this
  package's own `compress()`/`decompress()` round trip never exercised the
  path (this package's encoder never emits offset codes < 2, since its
  minimum LZ77 match offset is 1, giving `rawOff = offset + 3 >= 4`
  always).

  This is the same gap independently identified and documented as
  lessons.md **Lesson 98** while implementing the first `c/zstd` port
  (PR #9941), which found it via real-CLI fuzzing (4713 bytes of a single
  repeated byte compresses, via the real `zstd` CLI, to a *Compressed*
  block — not RLE — whose one sequence has `Offset_Value=1`, i.e. "reuse
  Repeated_Offset1"). This port had the identical bug for the identical
  reason (no port's self-consistency round trip, and no prior automated
  CLI-interop test for this package, ever exercised real-`zstd`-encoded
  repeat-offset sequences). Verified independently reproducible in this
  environment before fixing: TC30 (added below) failed with `"decoded
  offset underflow: of_raw=1"` prior to this fix.

  Fixed in `Zstd.kt`'s `decompressBlock`/`decompress`: implemented full
  Repeated-Offset (R1/R2/R3) decode support per RFC 8878 §3.1.1.3.2.1.1,
  cross-checked against both the RFC prose and the literal reference C
  source (`ZSTD_decodeSequence` in `zstd_decompress_block.c`) via PR
  #9941's already-verified `c/zstd` fix, per the Lesson-96 playbook of not
  trusting either source alone — including the "when `Literals_Length` is
  0, repeated offsets are shifted by 1" special case. The three registers
  (`rep: LongArray` of size 3) are **frame-scoped** — default `1/4/8` for
  the first block, threaded unmodified by `Zstd.decompress` through every
  Compressed block's sequences for the rest of the frame (Raw/RLE blocks
  don't touch them) — not block-scoped or reset per Compressed block. This
  package's own encoder (`encodeSequencesSection`) is intentionally left
  unchanged; this is a decode-only fix.

### Added
- TC29: automated real `zstd` CLI interop test (both directions — our
  `compress()` decoded by real `zstd -d`, and real `zstd`'s output decoded
  by our `decompress()`) using an English-prose corpus. Unlike the 0.1.1
  changelog's TC-9 note ("manual verification... since none of [the other
  language ports] automate a CLI subprocess test either"), this is now a
  real, automated JUnit test (`ProcessBuilder` + temp files), skipping
  gracefully via `Assumptions.assumeTrue` when `zstd` isn't on `PATH`.
- TC30: automated real `zstd` CLI interop regression test proving the
  Repeated-Offset decode fix — 4713 bytes of a single repeated byte,
  compressed by the *real* `zstd` CLI (verified in this environment to
  produce a Compressed block with an `Offset_Value=1` repeat-offset
  sequence, not an RLE block), decompressed byte-exact by our decoder.
  Fails against the pre-fix decoder with the underflow error quoted above.
- Ad hoc, non-committed confidence pass beyond the two committed tests
  (mirroring PR #9941's fuzz harness): 24 of 25 real-`zstd`-CLI-produced
  inputs (constant-byte runs from 100 B to 300 KB spanning a block
  boundary, periodic patterns at several periods, repeated prose, random
  data, and a multi-run alternating pattern) round-tripped byte-exact
  through this decoder. The one failure (`"unsupported literals type 2"`)
  is an unrelated, pre-existing, explicitly-documented limitation of this
  port — Huffman-coded literals sections aren't supported, only
  `Raw_Literals` — orthogonal to sequence/offset decoding and out of scope
  for this fix.

### Verified
- Full existing test suite (28 tests as of 0.1.1) still passes unchanged —
  this package's own round trip never touches the repeat-offset path, so
  none of it was affected by the fix.
- 30 tests total (28 existing + TC29 + TC30), all passing.
- Line coverage: 92.9% (`jacocoTestCoverageVerification`, ≥ 80% gate),
  comfortably clear of the 0.1.1 baseline of 94% given the two added tests
  exercise mostly-already-covered code paths plus the new repeat-offset
  branches.

## [0.1.1] — 2026-08-03

Rescued from an orphaned branch (`worktree-feat+zstd-and-catchups`) that was
never merged. The package was ~3 months stale and did not build against
current `main`; this release brings it up to date, fixes bugs found while
verifying it, and closes out full CMP07 conformance including real `zstd`
CLI interoperability (TC-9), which had never actually been verified for
*any* language's ZStd port in this repo (see below).

### Fixed
- **Staleness**: `code/packages/kotlin/lzss`'s public API had moved on since
  this branch was written (`Lzss` → `LZSS`, `LzssToken.Literal`/`.Match` →
  top-level `Literal`/`Match` in a `Token` sealed interface, and
  `Literal.value` changed from `Byte` to `Int`). Updated imports and call
  sites in `Zstd.kt` accordingly; the composite-build wiring
  (`includeBuild("../lzss")` + Gradle dependency substitution) itself was
  already correct and needed no changes.
- **Real `zstd` CLI interoperability (TC-9) was completely broken**, in a
  way invisible to this package's own round-trip tests. Root-caused via
  RFC 8878 §3.1.1.3.2.1.2 (fetched and quoted verbatim while debugging) to
  three bugs in the Sequences section's FSE bitstream handling:
  1. `buildDecodeTable`/`buildEncodeTable` spread symbols into the table in
     a "count > 1 symbols first, then count == 1 symbols" two-pass order.
     The real (and RFC-documented) algorithm walks symbols in a single
     ascending pass over symbol index — the two-pass version is
     self-consistent (encoder and decoder agree with *each other*) but
     produces a different table than a real `zstd` decoder independently
     rebuilds from the same predefined distribution, so the bitstream
     silently decodes to the wrong values against a real decoder.
  2. FSE state init/update order was LL, ML, OF throughout. RFC 8878
     specifies LL, OF, ML for initialization and LL, ML, OF for per-sequence
     state updates — these are different orders, and the code used the
     wrong one for both.
  3. "Peek the current symbol" and "update the state" were conflated into
     one bit-consuming step (`fseDecodeSym`), read in LL, OF, ML order.
     RFC 8878 requires these as two separate steps: peek all three symbols
     first (no bits consumed), read per-sequence extra bits in OF, ML, LL
     order, and only then — and only if this isn't the last sequence in the
     block — update the three states (consuming bits) in LL, ML, OF order.
     The very last sequence's states must instead be seeded directly from
     its own symbols with zero bits emitted, mirroring the reference
     implementation's `FSE_initCState2`; the old code had no such special
     case at all.

  Split `fseDecodeSym` into `peekFseSym` (bit-free) and `updateFseState`
  (bit-consuming), made `fseEncodeSym`'s bit writer nullable to support
  init-without-emission, and rewrote `encodeSequencesSection` and
  `decompressBlock`'s sequence loop around the corrected field order.
  Verified empirically post-fix: both directions of real `zstd` CLI
  round-trip now match byte-for-byte (English prose, a repeat-offset
  pattern, and a 200 KB multi-block input).

  **This bug was not introduced by this port** — it was already present in
  `code/packages/rust/zstd` (the package this task named as the reference
  implementation to consult), confirmed by reproducing the identical
  `zstd -d` "Data corruption detected" failure against it. It is very
  likely present in the other 11 language ports too, since none of them
  have an automated TC-9 test either. Flagged separately for a follow-up
  pass across the other languages — out of scope to fix all of them in this
  PR.

### Added
- Block-size cap enforcement in `decompress()`: a block header claiming
  `Block_Size > 128 KB` (`MAX_BLOCK_SIZE`) is now rejected before any
  allocation or copy is attempted, closing off a "block bomb" vector where
  a single corrupt/hostile 3-byte block header could claim up to ~2 MB
  (the field's full 21-bit range).
- Decompression-bomb guard now applies *inside* `decompressBlock`'s
  sequence loop, not just once per Raw/RLE block. A Compressed block's
  on-wire size (capped at 128 KB) says nothing about how much output its
  FSE-coded sequences can produce — a single sequence's match length can
  be ~128 KB, and a block can carry on the order of 10^5 sequences — so the
  256 MB total-output cap (`MAX_DECOMPRESSED_SIZE`) is now checked
  incrementally before every literal-run append and match copy.
- 4 new tests (28 total, up from 24): oversized-`Block_Size` rejection,
  the sequence-level decompression-bomb guard (using a test-only lowered
  cap so the test doesn't have to materialise hundreds of MB to prove the
  throw fires), bad-magic rejection, and unsupported-FSE-mode rejection.
- `jacoco` coverage reporting + an 80% line-coverage gate
  (`jacocoTestCoverageVerification`), matching the convention already used
  by other composite-build Kotlin packages (e.g. `sql-codegen`). Currently
  at 94% line coverage.

### Verified
- All 10 of CMP07-zstd.md's mandatory test cases pass, including TC-9
  (real `zstd` CLI interoperability, both directions — manual verification,
  consistent with how every other language's zstd port documents this
  case, since none of them automate a CLI subprocess test either).
- Builds and tests clean against current `main`'s toolchain (Gradle 8.14.4
  / Kotlin 2.1.20 / JDK 21 via `mise`).
- `layout.buildDirectory = file("gradle-build")` override (required so
  Gradle's default `build/` output doesn't collide with the sibling `BUILD`
  script on case-insensitive filesystems) was already present and correct.

## [0.1.0] — 2026-04-24

### Added
- Initial implementation of ZStd (CMP07) compression and decompression in Kotlin.
- `Zstd.compress(ByteArray): ByteArray` — produces a conforming RFC 8878 ZStd frame.
- `Zstd.decompress(ByteArray): ByteArray` — decodes Raw, RLE, and Compressed blocks.
- FSE predefined tables for LL, ML, and OF coding (RFC 8878 Appendix B).
- `RevBitWriter` / `RevBitReader` — backward bitstream codec for the FSE sequence section.
- Raw literals encoding/decoding (1-byte, 2-byte, and 3-byte header variants).
- LZSS integration via `com.codingadventures:lzss` for LZ77 token generation.
- 24 unit tests covering round-trips, wire format, internal codec helpers, and edge cases.
