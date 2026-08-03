# Changelog — kotlin/zstd

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

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
