# zstd — CMP07

Zstandard lossless compression in pure Rust.

## What it does

Compresses and decompresses bytes using the Zstandard algorithm (RFC 8878).
Output is a valid `.zst` frame: the `zstd` CLI and any RFC-8878-compliant
library can decompress what this crate produces (and vice-versa for raw and
RLE blocks; compressed blocks use the predefined FSE tables which all
conforming decoders support).

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

## Predefined tables

The predefined distributions (RFC 8878 Appendix B) allow zero table-description
overhead. This implementation uses only `Predefined_Mode` (mode byte = 0x00),
so it is compatible with all decoders that support predefined modes.

## Tests

```
cargo test -p zstd
```

25 unit tests + 3 doctests:

| Test | What it checks |
|------|----------------|
| `tc1_empty` | Empty input round-trip |
| `tc2_single` | Single byte |
| `tc3_all_bytes` | All 256 byte values |
| `tc4_rle` | 1024 identical bytes → < 30 bytes |
| `tc5_prose` | English text → ≥ 20% compression |
| `tc6_random` | Pseudo-random data (LCG) round-trip |
| `tc7_multiblock` | 200 KB → two blocks |
| `tc8_repeat_offset` | Pattern with offset matches → < 70% |
| `tc9_deterministic` | Same input → identical output |
| `tc10_wire_format` | Hand-built raw frame decoded correctly |
| `test_fse_*` | Single and two-sequence FSE round-trips |
| `test_literals_*` | 1-, 2-, 3-byte literals headers |
| `test_revbit*` | Backward bit-stream round-trip |
| `test_seq_count*` | Sequence count encoding |

## Dependencies

- [`lzss`](../lzss) — LZ77 token generation (CMP02)
