# coding_adventures_zstd

Zstandard (ZStd) lossless compression — CMP07.

A pure-Elixir implementation of RFC 8878 (Zstandard) that compresses and
decompresses binary data using LZ77 back-references and FSE (Finite State
Entropy) coding.

## Where it fits

```
CMP02 (LZSS,   1982) — LZ77 + flag bits        ← dependency
CMP07 (ZStd,   2015) — LZ77 + FSE; high ratio  ← this package
```

## Quick start

```elixir
iex> data = "the quick brown fox jumps over the lazy dog" |> String.duplicate(25)
iex> compressed = CodingAdventures.Zstd.compress(data)
iex> byte_size(compressed) < byte_size(data)
true
iex> {:ok, ^data} = CodingAdventures.Zstd.decompress(compressed)
```

## API

### `compress/1`

```elixir
@spec compress(binary()) :: binary()
```

Compresses any binary to a valid ZStd frame. The frame starts with the magic
number `0xFD2FB528` (little-endian) and contains one or more blocks.

Block type selection (per 128 KB chunk):
1. **RLE** — all bytes identical → 4 bytes total
2. **Compressed** (LZ77 + FSE) — if smaller than raw
3. **Raw** — verbatim fallback

### `decompress/1`

```elixir
@spec decompress(binary()) :: {:ok, binary()} | {:error, String.t()}
```

Decompresses a ZStd frame. Returns `{:error, reason}` for:
- Bad magic number
- Truncated frame or block
- Unsupported features (non-predefined FSE tables, Huffman literals)
- Output exceeding 256 MB (decompression bomb guard)

## Algorithm overview

### LZ77 tokenisation

The input is first tokenised by LZSS (CMP02) with a 32 KB sliding window.
Each token is either a literal byte or a `(offset, length)` back-reference.

### Sequence packing

Consecutive literals before each back-reference are grouped into a
_sequence_ `{ll, ml, off}`:
- `ll` = literal length (how many literal bytes precede this match)
- `ml` = match length (how many bytes to copy from history)
- `off` = match offset (1-indexed distance back in the output)

### FSE encoding

Each of the three code fields (LL, ML, OF) is entropy-coded using FSE:

1. The predefined normalised distributions (RFC 8878 Appendix B) define fixed
   decode tables — no table overhead in the bitstream.
2. Sequences are encoded in **reverse order** into a backward bitstream.
3. The decoder reads initial FSE states, then decodes sequences in forward order.

### Reverse bitstream

The FSE sequence bitstream is written backwards: the encoder writes the last
sequence first. A sentinel bit in the final byte marks the stream boundary.
This allows the decoder to initialise from the end and decode forward.

## Frame wire format

```
┌────────┬─────┬──────────────────────┬────────┬──────────────────┐
│ Magic  │ FHD │ Frame_Content_Size   │ Blocks │ [Checksum]       │
│ 4 B LE │ 1 B │ 8 B LE              │ ...    │ (not written)    │
└────────┴─────┴──────────────────────┴────────┴──────────────────┘
```

Each block header is 3 bytes (little-endian):
```
bit 0       = Last_Block flag
bits [2:1]  = Block_Type  (00=Raw, 01=RLE, 10=Compressed, 11=Reserved)
bits [23:3] = Block_Size
```

## Running tests

```sh
cd ../lzss && mix deps.get --quiet
cd ../zstd && mix deps.get --quiet && mix test --cover
```

Expected coverage: ≥ 90%.

## Development notes

- **Elixir reserved words** avoided as variable names: `after`, `and`, `catch`,
  `do`, `else`, `end`, `false`, `fn`, `in`, `nil`, `not`, `or`, `rescue`,
  `true`, `when`, `with`.
- **Sequence count encoding** follows RFC 8878 §3.1.1.1.3 exactly: the 2-byte
  form uses `byte0 = (count >> 8) | 0x80` so the decoder can distinguish it
  from the 1-byte form by testing `byte0 >= 128`.
- **Back-reference copying** uses binaries throughout for O(1) random access via
  `:binary.at/2` and `binary_part/3`, avoiding the O(n) cost of list indexing.
- **Overlap-safe copy** handles LZ77 run-length expansion (e.g., offset=1 length=10
  expands one byte into ten identical bytes) by wrapping the copy window modulo
  the match distance.
