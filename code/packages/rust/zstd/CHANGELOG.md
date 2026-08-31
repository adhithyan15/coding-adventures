# Changelog — zstd

## 0.2.0 — 2026-08-31

The headline of this release is that `decompress()` can now read `.zst`
files that real encoders actually produce. Before it, the decoder handled
exactly one shape of Compressed block — `Raw_Literals` plus all-Predefined
FSE modes — which happens to be the only shape *this crate's own encoder*
emits. Every self round-trip test passed while `decompress()` returned
"unsupported literals type 2" on nearly anything the `zstd` CLI produced.
That is the same blind spot as Lessons 96 and 98: an encoder and decoder
that only ever talk to each other cannot discover what they both fail to
implement.

### Added

- **Huffman-coded literals (RFC 8878 §4.2.1).** Both
  `Compressed_Literals_Block` (type 2, carrying a tree description) and
  `Treeless_Literals_Block` (type 3, reusing an earlier block's tree), in
  both the single-stream and 4-stream layouts. The 4-stream layout splits
  the literal run into quarters behind a 6-byte jump table so a decoder can
  run four serial Huffman chains concurrently; only three stream sizes are
  transmitted because the fourth is whatever is left.
- **Huffman tree descriptions (§4.2.1.1)**, in both representations: raw
  4-bit weights packed two per byte, and weights carried by a 2-state
  interleaved FSE stream whose symbol count is implied by where the
  bitstream runs out. Includes the *deduced final weight* — the last
  symbol's weight is never transmitted; it is recovered from the shortfall
  between the transmitted weights' Kraft sum and the next power of two, and
  a shortfall that is not itself a power of two is what identifies a corrupt
  description.
- **FSE table descriptions (§4.1.1)** — `read_fse_table_description`, with
  the self-synchronising variable-width count field (each count is drawn
  from the mass still unassigned, so field widths shrink as the description
  proceeds, and both sides derive the width from state they already share)
  and the 2-bit repeat field that collapses runs of absent symbols.
- **All four `Symbol_Compression_Mode`s (§3.1.1.3.2.1)** for each of the
  literal-length, offset and match-length tables: `Predefined`, `RLE` (one
  symbol, zero bits, an `accuracy_log` of 0), `FSE_Compressed` (description
  in-band) and `Repeat` (reuse the previous Compressed block's table). Each
  table's own `accuracy_log` now drives the width of its initial state read,
  rather than the fixed predefined constant.
- **`RLE_Literals_Block` (§3.1.1.2)**, previously rejected alongside the
  Huffman types.
- **`FwdBitReader`** — the forward, little-endian bit convention used by
  table descriptions, as distinct from the backward `RevBitReader` used by
  every payload stream. Conflating the two is the classic way to build a
  decoder that reads only its own output.
- **`FrameState`** — one type now answers "what survives a block boundary":
  the repeated-offset triple, the Huffman table, and the three sequence FSE
  tables. Previously only the first of these did, which is why Treeless
  literals and `Repeat_Mode` were unreachable.
- **Golden-vector suite** (`tests/vectors/*.zst`): seven frames produced by
  the real `zstd` CLI v1.5.7 and committed as bytes, embedded with
  `include_bytes!` and decoded unconditionally on every platform with no
  subprocess involved. They pin the decoder against frames this crate
  *cannot produce*, so they keep working on machines with no `zstd` binary.
  `regenerate_golden_vectors` (an `#[ignore]`d test) recreates them
  byte-for-byte and documents the exact command line for each.
- **`cli_interop_corpus_forces_huffman_and_fse_tables`**: real-CLI interop
  over a corpus chosen so the CLI *must* reach for the new paths — pseudo
  prose, skewed few-symbol alphabets, incompressible bytes, and inputs on
  both sides of the 128 KB block boundary — at levels `-1`, `-3` and `-19`,
  because different levels emit structurally different frames.
- **`cli_interop_streaming_frames_without_content_size`**: the same corpus
  piped through `zstd`'s STDIN, which makes it omit `Frame_Content_Size` and
  emit a `Window_Descriptor` instead. Every streaming library produces frames
  of that shape — including the one that writes Anki's `.colpkg` payloads —
  and a decoder can parse the file-shaped header perfectly while mis-parsing
  this one by exactly one byte.
- **Adversarial tests**: exhaustive truncation of every golden vector,
  systematic byte mutation of their header/table regions, and hand-built
  frames for specific malformed shapes (oversized `accuracy_log`, weights
  that leave a non-power-of-two code space, `Repeat_Mode` with nothing to
  repeat, an offset RLE symbol past 31, a bitstream with no sentinel byte,
  RLE literals claiming 1 MB). All must return `Err`; a panic here would be
  a denial of service on untrusted `.apkg`/`.colpkg` input, and an
  unrecoverable trap on `wasm32-unknown-unknown` (`panic = "abort"`).

### Changed

- **A missing `zstd` CLI is now a test FAILURE, not a silent skip.** The
  interop tests used to open with `if !is_zstd_cli_available() { return; }`,
  which made every cross-implementation check in the file a no-op on any
  machine without the binary — a gate-shaped nothing, and precisely the
  condition under which Lessons 95/96/98 shipped. `require_zstd_cli()` now
  panics with install instructions. The live-CLI tests are scoped to
  `#[cfg(unix)]`; on Windows the golden vectors carry the conformance gate
  instead, so no platform is left without a cross-implementation check.
- `RevBitReader` now tracks a signed bit budget (`remaining`), mirroring the
  `i64 offset` of RFC 8878's reference educational decoder. Over-reads are
  detectable instead of silently decoding zero-fill, which the 2-state
  Huffman-weight stream *requires* (it has no symbol count on the wire), and
  which lets the sequences and Huffman literal streams enforce the reference
  decoder's `BIT_endOfDStream` rule: a conforming stream ends EXACTLY.
- FSE distributions are validated before a decode table is built from them
  (`FseTable::from_norm`), and FSE state lookups are bounds-checked
  (`fse_cell`). `build_decode_table` silently produces a table with
  duplicated and unwritten cells when the counts do not sum to
  `2^accuracy_log`; reading such a table can compute an out-of-range state.
- Literals `Regenerated_Size` is capped at the 128 KB block maximum, which
  bounds both the allocation and the expansion an `RLE_Literals_Block` can
  claim from a single payload byte.
- **A declared `Frame_Content_Size` is now cross-checked against what the
  blocks actually produced.** It was previously read and discarded. Since
  this crate carries no xxHash64 and so cannot verify the trailing content
  checksum, that size is the *only* end-to-end check available: without it a
  structurally well-formed but semantically wrong frame returns short output
  silently. It doubles as a cheap decompression-bomb signal — a frame
  declaring 100 bytes and producing 200 MB is corrupt regardless of how valid
  each individual block looked. Streamed frames (which carry no size field at
  all) are unaffected.
- **A frame with a non-zero `Dictionary_ID` is now refused by name.** The
  Dictionary_ID field used to be skipped and decoding attempted anyway. But a
  dictionary pre-seeds the match history *and* all four entropy tables, so a
  dictionary frame may legitimately open with `Repeat_Mode` tables or a
  `Treeless_Literals_Block` and reference offsets pointing into content that
  was never in the frame. Pressing on produced a baffling
  "offset table uses Repeat_Mode but no previous table exists in this frame"
  — or, on a frame that happened not to trip a missing-state check, silently
  wrong bytes. A field that is present but zero still means "no dictionary"
  and decodes normally.

### Fixed

- **Decoder never implemented Repeated-Offset (R1/R2/R3) sequence decoding
  (RFC 8878 §3.1.1.3.2.1.1)** — `decompress_block` computed
  `offset = of_raw - 3` unconditionally for every sequence, treating offset
  codes 1/2/3 as literal (tiny, usually invalid) offsets instead of
  references into a 3-slot history of recently-used offsets. This crate's
  own encoder never emits offset codes below 4 by design (an explicit
  "no repeat-offset shortcuts" educational simplification —
  `raw_off = offset + 3 >= 4` always, since the minimum LZ77 match offset is
  1), so this crate's own `compress()`/`decompress()` round trip — and every
  existing unit/interop test, including `tc9_cli_interop` and the
  misleadingly-named `tc8_repeat_offset` (which only exercises OUR encoder's
  explicit-offset path on repetitive input, never the repeat-offset decode
  path) — never exercised this code at all. But real `zstd` encoders use
  repeat offsets constantly, especially on periodic or constant data; a
  decoder that only understands explicit offset codes fails on a large
  fraction of real-world `.zst` files. Found and root-caused while building
  the sibling `code/packages/c/zstd` port (PR #9941) via fuzzing against the
  real CLI — see lessons.md Lesson 98. Fixed by threading a frame-scoped
  `(rep1, rep2, rep3)` offset-history triple (default `1/4/8` at the start
  of a frame, persisting across Raw/RLE blocks, updated after every
  Compressed block's sequences — explicit-offset or repeat-offset alike)
  through `decompress()`/`decompress_block()`, and interpreting offset codes
  0/1 (`of_raw` in `{1, 2, 3}`) as a repeat-offset reference selected by
  `ll_is_zero + of_raw - 1`, including the RFC's "when Literals_Length == 0,
  the repeat-offset interpretation shifts by 1, and slot 3 means `rep1 - 1`"
  special case. Algorithm cross-checked against both RFC 8878 prose and the
  literal reference C source (`ZSTD_decodeSequence` in
  `zstd_decompress_block.c`, fetched directly rather than recalled from
  memory), matching the already-verified fix in `code/packages/c/zstd`. The
  encoder is intentionally left unchanged (still never emits repeat-offset
  codes — this is a decode-only fix, matching the educational subset's
  documented scope).
- **FSE sequences-section codec: three compounding RFC 8878 non-conformance
  bugs**, found via a repo-wide zstd conformance audit (companion fixes:
  java/zstd #9780, kotlin/zstd #9774) and confirmed with the minimal repro
  `compress("ababababab" * 3)` (one sequence: `ll=2, ml=28, offset=2`) —
  output that round-tripped against itself but that the real `zstd` CLI
  rejected as corrupt:
  1. `build_decode_table` / `build_encode_sym` used a fabricated two-pass
     symbol-spread split (all `count > 1` symbols first, then all
     `count == 1` symbols). The real algorithm
     (`FSE_buildDTable_internal`'s low-probability branch) is a single pass
     over symbols `0..maxSymbolValue`, placing each symbol's full count
     immediately when encountered.
  2. Per-sequence field order was wrong: a decoder must PEEK all three
     symbols (LL/ML/OF) from the current state first (no bits consumed),
     THEN read extra bits in order OF, ML, LL, THEN update states in order
     LL, ML, OF. The previous code combined peek-and-update into one step
     and got both the extras/updates relative order and the OF/ML
     sub-order wrong.
  3. The state-transition "update" is skipped for the LAST sequence in a
     block (no next sequence to prepare state for) — the encoder's
     first-processed (semantically last) sequence must get its starting
     state via a direct `FSE_initCState2`-style formula (new
     `fse_init_state` function), not a normal bit-flushing transition.
  All three bugs were self-cancelling under same-codebase round-trip
  testing (our own encoder and decoder always agreed with each other), so
  they were invisible to every existing unit test, including a dedicated
  low-level "encode/decode two sequences" FSE test.
- **Number_of_Sequences 2-byte wire encoding had the marker byte in the
  wrong position.** `encode_seq_count`/`decode_seq_count` treated the
  2-byte form as a plain little-endian `u16` with the high bit set (low
  byte first, marker+high byte second). The real format
  (`ZSTD_encodeSequences`) writes the marker+high byte FIRST
  (`(count >> 8) | 0x80`) and the low byte second. Any block with 128+
  sequences was misparsed by the real `zstd` CLI; only caught by adding
  real interop coverage that pushes past the 1-byte/2-byte boundary
  (`rt_cli_interop_high_sequence_count`).
- **Frame Header Descriptor `Content_Checksum_Flag` was read from bit 4
  instead of bit 2.** Verified empirically: `zstd -c file.txt` (checksum on
  by default) emits FHD byte `0x64`; `zstd -c --no-check file.txt` emits
  FHD byte `0x60` — the differing bit is bit 2. RFC 8878 §3.1.1.1 agrees:
  bit 4 is `Unused_bit`, bit 2 is `Content_Checksum_Flag`. `decompress()`
  now reads the correct bit and validates that the trailing 4-byte checksum
  (when present) isn't truncated.
- **Decompression-bomb guard was missing inside Compressed-block sequence
  application.** `decompress()` only checked the 256 MB output cap for Raw
  and RLE blocks; a Compressed block's wire size is capped at 128 KB but
  says nothing about how large it can LZ77-expand to (a single sequence's
  match length can be up to ~131 KB, and one block can carry tens of
  thousands of sequences). Added `check_output_budget` calls inside
  `decompress_block`'s per-sequence loop, checked before every literal-run
  and match-copy append.

### Added

- **`tc11_repeat_offset_cli_interop_constant_byte` / `tc11_repeat_offset_cli_interop_periodic`**:
  real `zstd` CLI interop tests targeting the Repeated-Offset decode gap
  above. The constant-byte case reproduces the exact Lesson 98 repro (4713
  bytes of a single repeated byte, which real `zstd` encodes as one
  Compressed block with a single Offset_Value=1 sequence — "reuse
  Repeated_Offset1") and fails with `decoded offset underflow: of_raw=1`
  against the pre-fix decoder, proving the gap was real before it was fixed.
  Verified beyond the committed suite via real `zstd`-CLI-produced files
  covering prose, a numeric ramp, log-like text, and semi-random content
  (several hit this crate's pre-existing, documented, out-of-scope
  limitations — Huffman literals, non-Predefined FSE modes — unrelated to
  this fix; every file that used only the supported feature subset decoded
  byte-exact).
- **Real `zstd` CLI interop test (`tc9_cli_interop`)**, per spec TC-9: shells
  out to the system `zstd` binary via `std::process::Command` to verify both
  directions — compress with this crate and decompress with `zstd -d`, and
  compress with `zstd` and decompress with this crate — round-trip
  byte-exact. This is the test that actually proves RFC 8878 conformance;
  its absence (not the algorithm bugs alone) was the root cause that let all
  of the above bugs ship undetected. (As of 0.2.0 a missing `zstd` binary is
  a hard failure rather than a silent no-op — see **Changed** above.)
- `rt_cli_interop_high_sequence_count`: additional real-CLI regression test
  covering the Number_of_Sequences 2-byte-encoding boundary (128+
  sequences in one block).
- `test_fse_many_sequence_roundtrip`: internal FSE-codec unit test covering
  multiple non-last state transitions in addition to the single-sequence
  `fse_init_state` path.

## 0.1.0 — 2026-04-24

### Added

- Initial implementation of the Zstandard compression algorithm (RFC 8878, CMP07).
- `compress(data: &[u8]) -> Vec<u8>`: encodes any byte slice into a valid ZStd frame.
- `decompress(data: &[u8]) -> Result<Vec<u8>, String>`: decodes any single-segment ZStd frame.
- Full ZStd frame layout: magic number, FHD byte, 8-byte FCS, blocks.
- Three block types:
  - **Raw** blocks for incompressible data.
  - **RLE** blocks for single-value runs (e.g., 1024 'A' bytes → 17 bytes total).
  - **Compressed** blocks using LZ77 back-references + FSE sequence coding.
- Predefined FSE tables for Literal Lengths, Match Lengths, and Offsets
  (from RFC 8878 Appendix B), so frames require no per-block table description.
- `RevBitWriter` / `RevBitReader`: backward bit-stream codec (last-written bits
  read first), matching the ZStd sequence bitstream convention.
- Raw_Literals section encoding/decoding with 1-, 2-, and 3-byte headers.
- Multi-block support for inputs larger than 128 KB.
- Manual wire-format test verifying the decoder against a hand-built raw-block frame.
- 25 unit tests + 3 doctests; all pass.
- Literate-programming comments throughout explaining ZStd internals from first
  principles.

### Implementation notes

- LZ77 token generation is delegated to the `lzss` crate (CMP02) via
  `lzss::encode(block, 32768, 255, 3)` — 32 KB window, max match 255, min match 3.
- FSE encode table uses index-order (not fill-order) position assignment to
  maintain the encode/decode symmetry invariant.
- Sequence FSE symbols are written in ML→OF→LL order so the backward bit-stream
  delivers them in LL→OF→ML decode order.
- Raw_Literals uses size_format 00 (1-byte), 01 (2-byte), or 11 (3-byte) per
  the spec; size_format 10 is also accepted on decode as equivalent to 00.
