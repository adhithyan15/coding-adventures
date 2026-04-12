# lzss — LZSS Lossless Compression Algorithm (Elixir)

LZSS (Lempel-Ziv-Storer-Szymanski, 1982) sliding-window compression with flag-bit token
disambiguation. Part of the CMP compression series in the coding-adventures monorepo.

## In the Series

| Spec  | Algorithm      | Year | Key Concept                              |
|-------|----------------|------|------------------------------------------|
| CMP00 | LZ77           | 1977 | Sliding-window backreferences            |
| CMP01 | LZ78           | 1978 | Explicit dictionary (trie), no window    |
| CMP02 | **LZSS**       | 1982 | LZ77 + flag bits, no wasted literals ← you are here |
| CMP03 | LZW            | 1984 | Pre-initialized dictionary; powers GIF  |
| CMP04 | Huffman Coding | 1952 | Entropy coding; prerequisite for DEFLATE |
| CMP05 | DEFLATE        | 1996 | LZ77 + Huffman; ZIP/gzip/PNG/zlib        |

## What LZSS Improves Over LZ77

LZ77 always emits `(offset, length, next_char)` — even a pure back-reference wastes 1 byte on a
trailing literal. LZSS eliminates this: tokens are either `Literal(byte)` (1 byte) or
`Match(offset, length)` (3 bytes), with a 1-byte flag disambiguating groups of 8 tokens.

Result: pure back-references shrink from 4 bytes to 3 bytes; literals shrink from 4 bytes to
1 byte. Compression improves ~25–50% on typical repetitive data vs LZ77.

## Usage

```elixir
alias CodingAdventures.LZSS

# One-shot compression / decompression
data = "hello hello hello world"
compressed = LZSS.compress(data)
LZSS.decompress(compressed)  # => "hello hello hello world"

# Token-level API
tokens = LZSS.encode(data)
LZSS.decode(tokens)  # => "hello hello hello world"

# Custom parameters
tokens = LZSS.encode(data, 2048, 128, 3)
```

## API

| Function | Description |
|----------|-------------|
| `encode/4` | Encode to token stream |
| `decode/2` | Decode token stream |
| `compress/4` | Encode + serialise |
| `decompress/1` | Deserialise + decode |
| `serialise_tokens/2` | Serialise token list to CMP02 binary |
| `deserialise_tokens/1` | Deserialise CMP02 binary to token list |
| `literal/1` | Create a literal token map |
| `match/2` | Create a match token map |

### Tokens

Tokens are plain maps:
- `%{kind: :literal, byte: integer}` — a raw byte value.
- `%{kind: :match, offset: integer, length: integer}` — back-reference into the window.

### Wire Format (CMP02)

```
Bytes 0–3:  original_length  (big-endian uint32)
Bytes 4–7:  block_count      (big-endian uint32)
Bytes 8+:   blocks
  Each block:
    [1 byte]  flag — bit i (LSB-first): 0 = literal, 1 = match
    [variable] up to 8 items:
                 flag=0: 1 byte  (literal value)
                 flag=1: 3 bytes (offset BE uint16 + length uint8)
```

### Parameters

| Parameter   | Default | Meaning |
|-------------|---------|---------|
| window_size | 4096    | Maximum lookback distance. |
| max_match   | 255     | Maximum match length. |
| min_match   | 3       | Minimum match length for a Match token. |

## Development

```bash
mix deps.get
mix test
mix test --cover
```

40+ tests, 0 failures.
