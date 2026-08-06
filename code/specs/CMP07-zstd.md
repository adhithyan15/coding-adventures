# CMP07 — ZStd (Zstandard)

## Overview

Zstandard (ZStd, 2016) is a real-time lossless compression algorithm developed by Yann Collet
at Meta/Facebook and standardised as RFC 8878 (March 2021). It combines an **LZ77-style
match-finder** with **Finite State Entropy (FSE)** coding, achieving compression ratios
competitive with zlib at 3–10× faster decompression speeds.

```
Series:
  CMP00 (LZ77,     1977) — Sliding-window backreferences.
  CMP01 (LZ78,     1978) — Explicit dictionary (trie).
  CMP02 (LZSS,     1982) — LZ77 + flag bits; no wasted literals.
  CMP03 (LZW,      1984) — LZ78 + pre-initialised alphabet; GIF.
  CMP04 (Huffman,  1952) — Entropy coding; prerequisite for DEFLATE.
  CMP05 (DEFLATE,  1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
  CMP06 (Brotli,   2013) — DEFLATE successor; HTTP/2 standard.
  CMP07 (ZStd,     2016) — FSE + LZ77; Linux kernel / npm / macOS.  ← YOU ARE HERE
  CMP08 (LZMA,     2001) — Range coding + LZ77; 7-Zip / XZ.
```

ZStd is used in:
- Linux kernel (btrfs, f2fs squashfs, zram, bpftool)
- Android OTA updates
- macOS Software Update
- npm `.tar.zst` packages (Node.js 22+)
- Git object storage (experimental)
- Meta / Facebook internal storage
- Databases: RocksDB, MySQL InnoDB (optional), ClickHouse

## Historical Context

Yann Collet published ZStd as open-source in 2015; RFC 8878 followed in 2021. ZStd's
central innovation is replacing Huffman coding (integer bit-widths per symbol) with
**tANS (table-based Asymmetric Numeral Systems)**, invented by Jarek Duda (2013). Meta's
implementation, called FSE (Finite State Entropy), allows fractional bit costs per symbol
and achieves compression closer to the Shannon entropy limit than Huffman.

Unlike DEFLATE/zlib, ZStd separates the LZ-match phase from the entropy-coding phase at
the *block* level, enabling parallel compression and incremental streaming.

## Key Concepts

### FSE (Finite State Entropy / tANS)

Huffman coding assigns each symbol an integer number of bits (minimum 1). For a symbol
with probability 0.9, Huffman needs 1 bit — but the theoretical minimum is −log₂(0.9) ≈
0.15 bits. This gap is entropy loss.

FSE eliminates it with a **finite state machine over a probability table**:

```
Analogy — Huffman vs FSE:
  Huffman: every symbol gets a fixed binary label; common symbols get short labels.
  FSE:     symbols "share" byte boundaries; a 90%-probable symbol might cost 0.15 bits
           on average by crossing byte boundaries fractionally via the state machine.
```

FSE encoding:
1. Build a **normalised frequency table** for the symbols (lit lengths, match lengths,
   offsets). Normalised counts sum to a power of 2 (the table size = 2^AccuracyLog).
2. Build a **spread table** — each symbol occupies `count[sym]` slots, assigned by a
   **single pass** over symbols in ascending order `0..maxSymbolValue`: for each symbol,
   place its full count immediately (step = `(table_size>>1) + (table_size>>3) + 3`,
   skipping any slot already claimed by a probability−1 symbol). This is
   `FSE_buildDTable_internal`'s low-probability branch in the reference C implementation —
   there is **no** "handle count>1 symbols first, then count==1 symbols" second pass; a
   fabricated two-pass split of that shape produces a different (but internally
   self-consistent) table layout that is silently non-conformant with real zstd. This bug
   shape — passing a language port's own round-trip tests while producing wire-incompatible
   output — was found independently in three ports; see lessons.md Lesson 96 for the full
   derivation and the exact per-sequence field order (peek LL/ML/OF → read extras OF, ML,
   LL → update states LL, ML, OF, skipping the update entirely for the last sequence in a
   block).
3. Encode backwards: each symbol takes the current state, looks up which new state to
   transition to, and emits the difference as bits.

FSE decoding (the fast direction):
```
state → (symbol, num_bits_to_read, baseline)
next_state = baseline + read(num_bits)
```

This is a pure table lookup per symbol — extremely cache-friendly and branch-free.

### Streaming Blocks

ZStd never compresses the entire input monolithically. Data is split into **blocks** of
at most 128 KB. Each block is independently typed:

```
Block types:
  00 = Raw        — uncompressed literal copy
  01 = RLE        — entire block is one repeated byte
  10 = Compressed — literals section + sequences section, FSE-encoded
  11 = Reserved
```

Benefits: incremental decoding, parallel compression, random access with skippable frames.

### Sequences (LZ77 Matches)

After literals are extracted, the LZ77 back-references are encoded as **sequences**:
each sequence is a triple `(literal_length, match_length, offset)`. Three separate FSE
streams encode these three fields simultaneously.

**Repeat Offsets:** ZStd tracks the three most-recently-used offsets in slots R1, R2, R3
(RFC 8878 §3.1.1.3.2.1.1). Referencing a recent offset costs only 2 bits instead of
encoding the full distance. The history is **frame-scoped**, not block-scoped: it starts
at `R1=1, R2=4, R3=8` for the first block of a frame and is threaded, unmodified across
Raw/RLE blocks, through every Compressed block's sequences for the rest of the frame.

The mapping from `Offset_Value` (the raw FSE-decoded value, `of_raw` below) to an actual
offset — including the one genuinely easy-to-miss special case — is:

```
Offset_Value ≥ 4        → actual offset = Offset_Value - 3 (explicit); then push it onto
                           the history: R3←R2, R2←R1, R1←new_offset
Offset_Value ∈ {1, 2, 3} → a repeat-offset reference, chosen by
                           selector = (Literals_Length == 0 ? 1 : 0) + Offset_Value - 1:
                             selector 0 → reuse R1 (no rotation)
                             selector 1 → use R2 (R1,R2 swap; R3 untouched)
                             selector 2 → use R3 (rotate: R1,R2,R3 ← new,old_R1,old_R2)
                             selector 3 → use R1-1 (same rotation shape as selector 2)
```

> **Correction (2026-08):** the history-selection rule is NOT simply "offset code N
> selects Rn" — that description (an earlier revision of this section) omits the
> `Literals_Length == 0` shift, which changes which slot codes 1/2/3 select and, for
> `Offset_Value == 3` with `Literals_Length == 0`, replaces the slot lookup entirely with
> `R1 - 1`. Every language port's DECODER in this repo initially implemented `offset =
> Offset_Value - 3` unconditionally for every code — including code 1/2/3 — because this
> repo's own ENCODERS never emit repeat-offset codes by design (an explicit "no
> repeat-offset shortcuts" educational simplification: every offset an encoder here
> writes is `>= 4` on the wire), so a self round trip never exercises this path, and even
> the spec's own TC-8 (built to be repeat-offset-*friendly* input, not repeat-offset-*
> aware* output) doesn't catch it. Real `zstd` encoders use repeat offsets constantly, so
> a decoder missing this fails to decode a large fraction of real-world `.zst` files. See
> `lessons.md` Lesson 98 and the `rust/zstd` / `c/zstd` conformance fixes; cross-checked
> against the reference C source (`ZSTD_decodeSequence` in `zstd_decompress_block.c`).
> Implementing this remains DECODE-only — encoders in this repo are not required to emit
> repeat-offset codes.

### ZStd vs DEFLATE vs Brotli

```
Property          DEFLATE     Brotli      ZStd
─────────────────────────────────────────────────
Ratio (text)      ~2.9×       ~3.3×       ~3.1×
Decomp speed      ~300 MB/s   ~300 MB/s   ~1000 MB/s
Comp speed        ~50 MB/s    ~5 MB/s     ~400 MB/s
Streaming         yes         yes         yes
Random access     no          no          skippable frames
Entropy coder     Huffman     Huffman     FSE (tANS)
Primary use       ZIP/gzip    HTTP        kernel/storage
```

## Wire Format (RFC 8878)

### Frame Layout

```
┌─────────────────────────────────────────────────────┐
│  Magic Number   (4 bytes) 0xFD2FB528 little-endian  │
│  Frame Header   (2–14 bytes)                        │
│  Block_0        (3-byte header + data)              │
│  Block_1        ...                                 │
│  Block_N        (Last_Block flag = 1)               │
│  [Content Checksum] (4 bytes, optional xxHash64)    │
└─────────────────────────────────────────────────────┘
```

Magic number bytes in file: `28 B5 2F FD`

### Frame Header Descriptor (FHD byte)

```
Bits 7–6:  Frame_Content_Size_Flag
           00 → FCS field absent (unknown size)
           01 → FCS is 2 bytes (value + 256)
           10 → FCS is 4 bytes
           11 → FCS is 8 bytes
Bits 5:    Single_Segment_Flag — if 1, no Window_Descriptor; FCS always present
Bit  4:    Unused_bit (decoders must ignore; not the checksum flag — see note below)
Bit  3:    Reserved (must be 0)
Bit  2:    Content_Checksum_Flag — if 1, 4-byte checksum appended after last block
Bits 1–0:  Dictionary_ID_Flag
           00 → no dictionary ID
           01 → 1-byte dict ID
           10 → 2-byte dict ID
           11 → 4-byte dict ID
```

> **Correction (2026-08):** an earlier revision of this table placed
> Content_Checksum_Flag at bit 4 and marked bit 2 as reserved. That was
> backwards. Verified empirically against the real `zstd` CLI: `zstd -c file`
> (checksum on by default) emits FHD `0x64`; `zstd -c --no-check file` emits
> FHD `0x60` — the differing bit is bit 2, and the checksummed output is
> exactly 4 bytes longer. RFC 8878 §3.1.1.1 agrees: bit 4 is Unused_bit, bit 2
> is Content_Checksum_Flag. A decoder that reads bit 4 for the checksum flag
> will misparse any real-world frame that has a trailing checksum (which is
> the common case — most encoders, including the reference `zstd` CLI, enable
> it by default) as having trailing garbage after the last block. This
> mistake was repo-wide (spec, Go, and Rust) — several language ports had
> already copied the wrong bit position into a code comment before this was
> caught; see `lessons.md` Lesson 95/96 and the `rust/zstd` conformance fix
> that corrected this table.

### Window Descriptor (1 byte, present when Single_Segment_Flag=0)

```
Bits 7–3: Exponent   (E)
Bits 2–0: Mantissa   (M)

Window_Size = (1 + M/8) × 2^(10 + E)
```

Minimum 1 KB, maximum 8 MB for decoders that claim conformance.
Educational implementations: fix Window_Size = 8 MB (E=13, M=0 → byte 0x68).

### Block Header (3 bytes)

```
Byte 0 bit 0:    Last_Block  (1 = this is the final block)
Byte 0 bits 2–1: Block_Type  (00=Raw, 01=RLE, 10=Compressed, 11=Reserved)
Bits 23–3:       Block_Size  (21-bit little-endian)

block_size = (byte[0] >> 3) | (byte[1] << 5) | (byte[2] << 13)
```

Raw block: `Block_Size` bytes of literal data follow.
RLE block: 1 byte follows; repeated `Block_Size` times.
Compressed block: `Block_Size` bytes of compressed content follow.

### Compressed Block Layout

```
┌───────────────────────────────┐
│  Literals_Section             │
│    Header (1–5 bytes)         │
│    [Huffman_Tree_Description] │
│    Compressed/Raw literals    │
├───────────────────────────────┤
│  Sequences_Section            │
│    Sequences_Count (1–3 bytes)│
│    Symbol_Compression_Modes   │  ← 1 byte: 2 bits each for LL/OF/ML
│    FSE tables (variable)      │
│    Bit-stream (backwards)     │
└───────────────────────────────┘
```

**Literals Header types:**
```
bits 1–0 = 00: Raw literals — size in bits 7–2 (up to 32 bytes inline)
bits 1–0 = 01: RLE literals — 1 byte repeated; size in bits 7–2
bits 1–0 = 10: Compressed (Huffman) literals
bits 1–0 = 11: Treeless (Huffman, reuse previous tree)
```

**Symbol Compression Modes byte** (1 byte in Sequences_Section):
```
bits 7–6: Literal_Lengths_Mode   (0=predefined, 1=RLE, 2=FSE, 3=repeat)
bits 5–4: Offsets_Mode
bits 3–2: Match_Lengths_Mode
bits 1–0: Reserved
```

### Predefined FSE Tables

ZStd ships with built-in default distributions used when no custom table is present
(`mode = 0`). Implementations must hardcode these exactly per RFC 8878 Appendix B:

- **Literal Lengths**: AccuracyLog=6, 36 symbols
- **Match Lengths**: AccuracyLog=6, 53 symbols
- **Offsets**: AccuracyLog=5, 29 symbols

### Sequence Encoding

Each sequence is encoded as three parallel FSE streams (interleaved, backward):
```
Literal_Length_Code → actual literal length via defined table
Match_Length_Code   → actual match length + 3 (min match = 3)
Offset_Code         → actual offset (see repeat offset rules above)
```

The bit streams are written **right-to-left** (backwards) and decoded left-to-right.
This allows the encoder to determine state transitions without lookahead.

#### Exact decode algorithm (bit-consumption order)

This is the part every language port in this repo got wrong on the first pass
(discovered during a repo-wide zstd conformance audit, 2026-08 — first found
during the `java/zstd` rescue and reproduced identically against the Rust
reference and multiple other independent implementations across this repo —
see `lessons.md`). It is NOT recoverable from a same-codebase round-trip
test: an encoder/decoder pair that agree with each other on a wrong order
will still round-trip correctly against themselves. It is only detectable by
decoding with (or encoding for) an independent, spec-conformant
implementation — i.e. `zstd -d` on the CLI (TC-9, aka test case 9). Verified
against RFC 8878 §3.1.1.3.2.1.2 and the actual
reference C source (`ZSTD_decodeSequence`, `FSE_encodeSymbol`,
`FSE_initCState2` in `github.com/facebook/zstd`):

1. **Initial states.** Three reads at the start of the bitstream, each
   `AccuracyLog` bits: **Literals_Length_State, Offset_State,
   Match_Length_State** — in that order. This becomes the state used to
   decode sequence 1.
2. **Per sequence**, repeated for every sequence in the block:
   a. **Peek** all three symbols from the CURRENT states —
      `symbol = table[state].symbol`. This is a bare table lookup; it
      consumes **zero bits**. (This is the detail every port missed: it's
      tempting to fold "get symbol" and "consume transition bits" into one
      step, but real zstd keeps them separate, and the separation is load-
      bearing for step (c) below.)
   b. **Read extra value bits**, in order **Offset, Match_Length,
      Literals_Length**. (Note: this is the REVERSE of the initial-state
      order in step 1, and also the reverse of the state-update order in
      step (c) — the RFC is genuinely asymmetric here, not a typo.)
   c. **Update FSE states** (consumes bits: `new_state = table[old_state].base
      + read(table[old_state].nbBits)`), in order **Literals_Length,
      Match_Length, Offset** — but **only if this is not the last sequence
      in the block**. There is no "next" sequence to prepare a state for
      after the last one, so no update bits are read (and a conformant
      encoder must not write any either — see below).
3. No content checksum / trailing bytes may follow the last block's payload
   (`Content_Checksum_Flag`, if set, contributes exactly 4 more bytes before
   end-of-frame; anything after that is a malformed/truncated frame and
   must be rejected, not silently ignored).

**Encoder implications** (the above is what a forward decoder consumes; the
encoder writes it in exact reverse, since the bitstream is backward): process
sequences from last to first. For the FIRST sequence processed (semantically
the LAST real sequence), there is no bit-consuming transition to produce its
starting state — it must be computed directly from the symbol with **zero
bits written**, using the same formula real zstd calls `FSE_initCState2`:
given per-symbol encode-table entries `deltaNbBits`/`deltaFindState` and the
`stateTable`,
```
nbBitsOut = (deltaNbBits + (1 << 15)) >>> 16
value     = (nbBitsOut << 16) - deltaNbBits
state     = stateTable[(value >>> nbBitsOut) + deltaFindState]
```
For every subsequent sequence processed (all but the first), the normal
`FSE_encodeSymbol` transition applies: `nbBitsOut = (state + deltaNbBits) >>>
16`, flush the low `nbBitsOut` bits of `state`, then
`state = stateTable[(state >>> nbBitsOut) + deltaFindState]`.

#### Exact FSE table-construction algorithm

Also load-bearing, also invisible to self-round-trip tests: the symbol
"spread" step that assigns table slots must be a **single pass** over
symbols in ascending order (`for s in 0..maxSymbolValue: place
normalizedCount[s] copies of s, advancing by `step = (tableSize>>1) +
(tableSize>>3) + 3` positions each time, skipping any position already
claimed by a -1-probability symbol`). There is no "handle count>1 symbols in
one pass, then count==1 symbols in a second pass" — that is NOT part of the
real algorithm; it produces a different (but internally self-consistent)
table and was one of several such bugs found independently across this
repo's language ports (first in `java/zstd`'s first pass). See
`FSE_buildDTable_internal` in `github.com/facebook/zstd`'s `fse_decompress.c`
for the canonical version.

#### Number_of_Sequences wire encoding

The 2-byte form of `Number_of_Sequences` (values 128–32511) is **not** a
plain little-endian `u16` with the high bit set — the marker/high byte comes
FIRST on the wire regardless of host endianness: `byte0 = (count >> 8) |
0x80`, `byte1 = count & 0xFF`. A little-endian-style encoding that writes the
low byte first is self-consistent (round-trips against itself) but produces a
non-conformant frame for any block with 128+ sequences.

## Educational Simplification

The educational implementation **must produce and consume valid .zst files**
(interoperable with `zstd -d` on the command line) but may simplify:

| Feature | Full ZStd | Educational |
|---------|-----------|-------------|
| Literals | Huffman or raw | Raw only |
| Sequences FSE | Custom + predefined tables | Predefined tables only |
| Block types | Raw / RLE / Compressed | All three |
| Dictionary | Yes | No |
| Skippable frames | Yes | No (skip on read) |
| Checksums | Optional | Omit (flag=0) |
| Window size | Up to 8 MB | Fixed 8 MB |

Raw literals + predefined FSE tables for sequences produces valid, decompressible output.
The interoperability requirement (test case 9) ensures the format is real, not toy.

## Public API

```
compress(data: bytes) → bytes
decompress(data: bytes) → bytes
```

Same interface as CMP00–CMP06.

## Package Naming

| Language   | Package name                 | Module / namespace             |
|------------|------------------------------|--------------------------------|
| Python     | `coding-adventures-zstd`     | `coding_adventures_zstd`       |
| Go         | module `…/go/zstd`           | package `zstd`                 |
| Ruby       | `coding_adventures_zstd`     | `CodingAdventures::Zstd`       |
| TypeScript | `@coding-adventures/zstd`    | `CodingAdventures.Zstd`        |
| Rust       | `coding-adventures-zstd`     | `coding_adventures_zstd`       |
| Elixir     | `:coding_adventures_zstd`    | `CodingAdventures.Zstd`        |
| Lua        | `coding-adventures-zstd`     | `coding_adventures.zstd`       |
| Perl       | `CodingAdventures::Zstd`     | `CodingAdventures::Zstd`       |
| Swift      | `CodingAdventuresZstd`       | `CodingAdventures.Zstd`        |

## Test Cases

All 10 test cases are mandatory. Every language implementation must pass them all.

### TC-1: Round-trip empty input
```
input  = b""
output = decompress(compress(b""))
assert output == b""
```

### TC-2: Round-trip single byte
```
input  = b"\x42"
output = decompress(compress(b"\x42"))
assert output == b"\x42"
```

### TC-3: Round-trip all 256 byte values
```
input = bytes(range(256))       # one of each byte 0x00–0xFF
output = decompress(compress(input))
assert output == input
# Note: incompressible data — compressed may be larger than input; round-trip must be exact
```

### TC-4: Round-trip highly repetitive data
```
input = b"A" * 1024
output = decompress(compress(input))
assert output == input
assert len(compress(input)) < 30   # RLE or match sequence; must compress dramatically
```

### TC-5: Round-trip English prose
```
text = "the quick brown fox jumps over the lazy dog " * 25  # ~1100 bytes
input = text.encode("utf-8")
output = decompress(compress(input))
assert output == input
assert len(compress(input)) < len(input) * 0.80   # must achieve at least 20% compression
```

### TC-6: Round-trip binary blob
```
# 512 bytes from a deterministic PRNG (LCG: seed=42, a=1664525, c=1013904223, m=2^32)
seed = 42
data = []
for _ in range(512):
    seed = (seed * 1664525 + 1013904223) & 0xFFFFFFFF
    data.append(seed & 0xFF)
input = bytes(data)
output = decompress(compress(input))
assert output == input   # ratio requirement: none — pseudo-random data is incompressible
```

### TC-7: Multi-block frame
```
input = b"x" * (200 * 1024)    # 200 KB — forces at least 2 blocks (block max = 128 KB)
output = decompress(compress(input))
assert output == input
# Verify more than one block was produced (inspect wire format or use a counter)
```

### TC-8: Repeat-offset compression
```
# Build a string where the same 8-byte pattern appears at the same distance repeatedly
pattern = b"ABCDEFGH"
input = pattern + (b"X" * 128 + pattern) * 10    # pattern at offset 128 every time
compressed = compress(input)
output = decompress(compressed)
assert output == input
# The repeat-offset mechanism should make this compress efficiently
assert len(compressed) < len(input) * 0.70
```
> Note: this test only requires the compressed round trip to be correct and reasonably
> small — it does NOT, by itself, prove an implementation's *decoder* understands
> repeat-offset sequences (offset codes 1/2/3), since an implementation whose own encoder
> never emits repeat-offset codes at all (this repo's educational encoders don't) can pass
> TC-8 while still failing to decode a real repeat-offset-aware encoder's output. See the
> Repeat Offsets correction above (Lesson 98) — that gap is only caught by real
> cross-implementation interop (TC-9-style), not by this test.

### TC-9: Cross-language / interoperability
```
# Compress with the standard `zstd` CLI (or reference implementation), decompress with ours
# Compress with ours, decompress with the standard `zstd -d` CLI
text = "the quick brown fox jumps over the lazy dog " * 25
# Both directions must round-trip exactly
```
*This test is manual or implemented via subprocess in CI where the `zstd` CLI is available.*

### TC-10: Wire format — minimal raw-block frame
```
# Manually construct: Magic + FHD(content_size=5, no checksum, no dict) +
#                     Window_Descriptor + Block(Last=1, Type=Raw, Size=5) + b"hello"
frame = bytes([
    0x28, 0xB5, 0x2F, 0xFD,   # magic
    0x60,                      # FHD: FCS_flag=11 (8-byte) → actually use minimal form
    # ... (see RFC 8878 §3.1.1 for exact byte layout)
])
# Exact bytes depend on the FHD flags chosen; document the expected byte sequence in tests.
# The point: construct a valid frame by hand and verify our decompress() handles it.
assert decompress(frame) == b"hello"
```

## Security Considerations

- **Bomb protection**: the `Content_Size` field is an untrusted hint — do not pre-allocate
  `Content_Size` bytes; grow output incrementally. Enforce a hard cap on total decompressed
  output size, checked incrementally at every point output can grow — including inside the
  per-sequence loop of Compressed-block decoding, not just once per top-level Raw/RLE block.
  A Compressed block's *wire* size is capped (see Block size cap below), but that says
  nothing about how large it can LZ77-expand to: a single sequence's match length can be
  tens of KB, and one block can carry tens of thousands of sequences.
- **Block size cap**: reject blocks claiming `Block_Size > 1 << 17` (128 KB + 1) as malformed.
- **FSE table validation**: verify that normalised counts sum to the declared table size;
  reject tables with negative counts or counts that overflow.
- **Offset bounds**: during sequence decoding, verify `offset ≤ bytes_decoded_so_far`
  before copying; out-of-bounds offsets must return an error, not a panic.
- **Sequence count**: `Sequences_Count` is a 24-bit field (≤ 8 388 607) — validate before
  allocating a sequence table.
- **Max recursion**: ZStd has no recursive structure; no stack-depth risk.
