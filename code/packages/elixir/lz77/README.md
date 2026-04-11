# lz77 — LZ77 Lossless Compression Algorithm (Elixir)

LZ77 sliding-window compression algorithm (Lempel & Ziv, 1977). Part of the CMP compression series in the coding-adventures monorepo.

## In the Series

| Spec  | Algorithm      | Year | Key Concept                              |
|-------|----------------|------|------------------------------------------|
| CMP00 | **LZ77**       | 1977 | Sliding-window backreferences ← you are here |
| CMP01 | LZ78           | 1978 | Explicit dictionary (trie), no window    |
| CMP02 | LZSS           | 1982 | LZ77 + flag bits, no wasted literals     |
| CMP03 | LZW            | 1984 | Pre-initialized dictionary; powers GIF  |
| CMP04 | Huffman Coding | 1952 | Entropy coding; prerequisite for DEFLATE |
| CMP05 | DEFLATE        | 1996 | LZ77 + Huffman; ZIP/gzip/PNG/zlib        |

## Usage

```elixir
alias CodingAdventures.LZ77

# One-shot compression / decompression
data = "hello hello hello world"
compressed = LZ77.compress(data)
LZ77.decompress(compressed)  # => "hello hello hello world"

# Token-level API
tokens = LZ77.encode(data)
LZ77.decode(tokens)  # => "hello hello hello world"

# Custom parameters
tokens = LZ77.encode(data, 2048, 128, 3)
```

## API

| Function | Description |
|----------|-------------|
| `encode/4` | Encode to token stream |
| `decode/2` | Decode token stream |
| `compress/4` | Encode + serialise |
| `decompress/1` | Deserialise + decode |
| `serialise_tokens/1` | Serialise token list to binary |
| `deserialise_tokens/1` | Deserialise binary to token list |
| `token/3` | Create a token map |

### Token

Tokens are plain maps: `%{offset: integer, length: integer, next_char: integer}`.

### Parameters

| Parameter   | Default | Meaning |
|-------------|---------|---------|
| window_size | 4096    | Maximum lookback distance. |
| max_match   | 255     | Maximum match length. |
| min_match   | 3       | Minimum match length for backreference. |

## Development

```bash
mix deps.get
mix test
mix test --cover
```

27 tests, 0 failures.
