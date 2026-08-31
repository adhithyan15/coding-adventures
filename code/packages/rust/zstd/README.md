# zstd — CMP07

Zstandard lossless compression in pure Rust.

## What it does

Compresses and decompresses bytes using the Zstandard algorithm (RFC 8878).

The two halves are deliberately asymmetric, and it is worth being explicit
about why:

- The **encoder** emits a narrow, easy-to-follow subset — Raw literals,
  predefined FSE tables, explicit offsets. Its output is a valid `.zst`
  frame that the real `zstd` CLI and any conforming library decodes.
- The **decoder** accepts the format as real encoders actually use it:
  Huffman-coded literals (treeless and 4-stream forms included), in-band
  FSE table descriptions, RLE and repeat table modes, RLE literal blocks,
  and repeated offsets.

That asymmetry is the point. A codec whose encoder and decoder only ever
talk to each other proves nothing about the wire format — this crate shipped
four separate conformance bugs behind exactly that illusion. So the decoder
is developed and tested against frames it *cannot produce*: live `zstd` CLI
interop, plus committed golden vectors for machines that have no `zstd`
binary.

## Where it fits

```
CMP00 (LZ77)      — Sliding-window back-references
CMP01 (LZ78)      — Explicit dictionary (trie)
CMP02 (LZSS)      — LZ77 + flag bits               ← lzss crate, used here
CMP03 (LZW)       — LZ78 + pre-initialised alphabet
CMP04 (Huffman)   — Entropy coding
CMP05 (DEFLATE)   — LZ77 + Huffman; ZIP/gzip/PNG
CMP06 (Brotli)    — DEFLATE + context modelling
CMP07 (ZStd)      — LZ77 + FSE; high ratio + speed  ← this crate
```

ZStd improves on DEFLATE by:
- Using **FSE (Finite State Entropy)** instead of Huffman coding for the
  sequence metadata — FSE approaches the theoretical entropy limit in a
  single pass and is branchless-friendly.
- Separating **literals** (raw bytes) from **sequences** (LZ77 back-refs)
  so each can be coded optimally.
- Framing that includes the uncompressed size, enabling single-alloc output
  buffers.

## Usage

```rust
use zstd::{compress, decompress};

let data = b"the quick brown fox jumps over the lazy dog";
let compressed = compress(data);
assert_eq!(decompress(&compressed).unwrap(), data);
```

## Compression pipeline

```
input bytes
    │
    ▼
lzss::encode()   — LZ77 sliding-window (32 KB, max match 255)
    │ Token stream: Literal(byte) | Match{offset, length}
    ▼
tokens_to_seqs() — group consecutive literals; emit (ll, ml, off) sequences
    │ lits: Vec<u8>   seqs: Vec<Seq{ll, ml, off}>
    ▼
encode_literals_section()   — Raw_Literals header + literal bytes
    │
encode_sequences_section()  — FSE bitstream (predefined tables, backward)
    │
block type selection: RLE < Compressed < Raw
    │
ZStd frame: Magic + FHD + FCS + Blocks
```

## Wire format (RFC 8878)

```
Frame:
  [28 B5 2F FD]  Magic (4 bytes, LE)
  [E0]           FHD: Single_Segment=1, FCS=8bytes, no checksum, no dict
  [xx .. xx]     Frame_Content_Size (8 bytes, LE u64)
  [Block] ...    One or more blocks

Block header (3 bytes, LE):
  bit 0      = Last_Block
  bits [2:1] = Block_Type  (00=Raw, 01=RLE, 10=Compressed)
  bits [23:3] = Block_Size
```

## FSE overview

FSE (Asymmetric Numeral Systems) is a range-coder variant. The encoder
maintains a state `S ∈ [sz, 2·sz)` and for each symbol `s`:

1. Compute `nb = (S + Δ_nb) >> 16` (symbol-specific transform).
2. Write the low `nb` bits of `S`.
3. Update `S = state_table[(S >> nb) + Δ_fs]`.

The decoder mirrors this using a lookup table of `(sym, nb, base)` triples:

1. Output `sym = table[S].sym`.
2. Read `nb = table[S].nb` bits.
3. Update `S = table[S].base + bits`.

The "backward" bitstream means sequences are encoded in reverse (last
sequence first), and the decoder reads them forward. Initial FSE states are
flushed as the last thing written, so the decoder reads them first.

## Table modes

Every entropy-coded field in ZStd can describe its table four ways, trading
description size against adaptivity. The encoder here always writes
`Predefined`; the decoder implements all four.

| Mode | Name | Wire cost | Meaning |
|------|------|-----------|---------|
| 0 | `Predefined` | 0 bytes | RFC 8878 Appendix B's fixed distribution |
| 1 | `RLE` | 1 byte | one symbol every time, costing zero bits |
| 2 | `FSE_Compressed` | variable | distribution described in-band (§4.1.1) |
| 3 | `Repeat` | 0 bytes | reuse the previous block's table |

A small block cannot afford mode 2 — the description would cost more than
the sequences it describes — which is why the other three exist, and why a
decoder implementing only mode 0 fails on small real-world files just as
badly as on large ones.

## Literals

```
Literals_Block_Type (bits [1:0] of the section's first byte):

  0  Raw         literal bytes verbatim
  1  RLE         one byte, repeated Regenerated_Size times
  2  Compressed  Huffman tree description, then Huffman bitstream(s)
  3  Treeless    Huffman bitstream(s), reusing the PREVIOUS block's tree
```

Types 2 and 3 come in a single-stream form and a **4-stream** form. Huffman
decoding is serial — symbol `n+1` cannot start until symbol `n`'s length is
known — so ZStd splits the literal run into quarters, each with its own
independent bitstream, behind a 6-byte jump table holding three little-endian
`u16` sizes. Only three are transmitted; the fourth stream's size is whatever
is left over.

The tree description transmits **weights**, not code lengths: a symbol of
weight `w > 0` gets a code of length `max_bits + 1 - w`, so a bigger weight
means a shorter code. Weights are small dense integers that compress well.
The last symbol's weight is never transmitted at all — it is recovered from
the shortfall between the transmitted weights' Kraft sum and the next power
of two.

## What survives a block boundary

ZStd blocks are deliberately not independent. Three things carry forward
within a frame, and a decoder that resets any of them mis-decodes most real
files:

1. **Repeated offsets** — a 3-slot history of recent match offsets (seeded
   at 1/4/8), so periodic data can say "same distance as last time".
2. **The Huffman table** — what a `Treeless_Literals_Block` reuses.
3. **The three sequence FSE tables** — what `Repeat_Mode` reuses.

## Tests

```
cargo test -p zstd
```

47 unit tests + 3 doctests. Grouped by what they can and cannot prove:

| Test group | What it checks | Can it catch a wire-format bug? |
|------------|----------------|---------------------------------|
| `tc1`–`tc8`, `rt_*` | Self round-trip: sizes, ratios, byte fidelity | **No** — encoder and decoder can be wrong in the same way |
| `test_fse_*`, `test_literals_*`, `test_revbit*`, `test_seq_count*` | Isolated codec units | **No** — same blindness, one level down |
| `fwd_bit_reader_*`, `fse_table_description_*`, `huffman_*` | The new table-description parsers against hand-built bit patterns | Partly — pins the parse, not the convention |
| `tc9_cli_interop`, `tc11_*`, `cli_interop_*` | Live `zstd` CLI, both directions, file-shaped and streamed frames, levels `-1`/`-3`/`-19` | **Yes** — the real oracle |
| `golden_vectors_decode_exactly` | Seven committed real-CLI frames, `include_bytes!`-embedded | **Yes** — and with no binary needed |
| `malformed_*`, `degenerate_*`, `oversized_*` | Truncation, byte mutation, hand-built corrupt frames | Robustness, not conformance |

The live-CLI tests **require** the `zstd` binary: a missing one fails the
test rather than skipping it. The earlier `if !available { return; }` made
every cross-implementation check a silent no-op, which is how three
wire-format bugs shipped. They are `#[cfg(unix)]`; on Windows the golden
vectors carry the same conformance gate without a subprocess.

To rebuild the golden vectors after an intentional change:

```
cargo test -p zstd -- --ignored regenerate_golden_vectors
```

## Dependencies

- [`lzss`](../lzss) — LZ77 token generation (CMP02)
