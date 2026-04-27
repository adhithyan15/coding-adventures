# lzw — LZW Lossless Compression Algorithm (Elixir)

LZW (Lempel-Ziv-Welch, 1984) dictionary-based compression with variable-width
bit-packing. The algorithm that powers GIF image compression. Part of the CMP
compression series in the coding-adventures monorepo.

## In the Series

| Spec  | Algorithm      | Year | Key Concept                                      |
|-------|----------------|------|--------------------------------------------------|
| CMP00 | LZ77           | 1977 | Sliding-window backreferences                    |
| CMP01 | LZ78           | 1978 | Explicit dictionary (trie), no window            |
| CMP02 | LZSS           | 1982 | LZ77 + flag bits; no wasted literals             |
| CMP03 | **LZW**        | 1984 | Pre-initialized dictionary; powers GIF ← you are here |
| CMP04 | Huffman Coding | 1952 | Entropy coding; prerequisite for DEFLATE         |
| CMP05 | DEFLATE        | 1996 | LZ77 + Huffman; ZIP/gzip/PNG/zlib                |

## What LZW Improves Over LZ78

LZ78 must always append a `next_char` byte after every code, because the
decoder might encounter a prefix it hasn't seen. LZW pre-seeds the dictionary
with all 256 single-byte sequences, so that "next_char" is always already
present — the encoder only ever transmits codes.

With pure codes to transmit, LZW can use variable-width bit-packing: codes
start at 9 bits and grow as the dictionary expands, matching GIF compression
exactly.

## Usage

```elixir
alias CodingAdventures.LZW

# One-shot compression / decompression
data = "hello hello hello world"
compressed = LZW.compress(data)
LZW.decompress(compressed)  # => "hello hello hello world"

# Code-level API
codes = LZW.encode_codes(data)
# => [256, 104, 101, 108, 108, 111, ...]
LZW.decode_codes(codes)  # => "hello hello hello world"

# Bit packing / unpacking
packed = LZW.pack_codes(codes, byte_size(data))
{codes_back, original_length} = LZW.unpack_codes(packed)
```

## API

| Function | Description |
|----------|-------------|
| `compress/1` | Encode + bit-pack to CMP03 wire format |
| `decompress/1` | Unpack + decode from CMP03 wire format |
| `encode_codes/1` | Encode binary to LZW code list |
| `decode_codes/1` | Decode LZW code list to binary |
| `pack_codes/2` | Bit-pack code list into binary |
| `unpack_codes/1` | Unpack binary into code list |

### Reserved Codes

| Code | Meaning |
|------|---------|
| 0–255 | Pre-seeded single-byte entries |
| 256 | `CLEAR_CODE` — reset dictionary |
| 257 | `STOP_CODE` — end of stream |
| 258+ | Dynamically added entries |

### Wire Format (CMP03)

```
Bytes 0–3:  original_length  (big-endian uint32)
Bytes 4+:   variable-width bit-packed codes, LSB-first
```

Codes start at 9 bits wide and grow by 1 bit each time the dictionary crosses
a power-of-2 boundary (512, 1024, …, 65536). The decoder mirrors this sizing
exactly, so both sides always agree on the current code width without any
out-of-band signalling.

### decompress/1 Error Handling

`decompress/1` returns `{:error, :too_short}` for inputs fewer than 4 bytes
(missing header), instead of crashing. All other malformed inputs are handled
gracefully (invalid codes are skipped, truncated streams return partial output).

## Development

```bash
mix deps.get
mix test
mix test --cover
```

50+ tests, 0 failures.
