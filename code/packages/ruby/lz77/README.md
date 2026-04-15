# lz77 — LZ77 Lossless Compression Algorithm (Ruby)

LZ77 sliding-window compression algorithm (Lempel & Ziv, 1977). Part of the CMP compression series in the coding-adventures monorepo.

## What Is LZ77?

LZ77 replaces repeated byte sequences with compact backreferences into a sliding window of recently seen data. It is the foundation of DEFLATE, gzip, PNG, and zlib.

A token `(offset, length, next_char)` means:
- **offset**: how many bytes back the match begins (1 = immediately before)
- **length**: how many bytes the match covers (0 = no match, emit literal)
- **next_char**: the literal byte that follows the match (always emitted)

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

```ruby
require "coding_adventures_lz77"

# One-shot compression / decompression
data = "hello hello hello world"
compressed = CodingAdventures::LZ77.compress(data)
original   = CodingAdventures::LZ77.decompress(compressed)

# Token-level API
tokens  = CodingAdventures::LZ77.encode(data)
decoded = CodingAdventures::LZ77.decode(tokens)

# Custom parameters
tokens = CodingAdventures::LZ77.encode(data, window_size: 2048, max_match: 128, min_match: 3)
```

## API

| Method | Signature | Description |
|--------|-----------|-------------|
| `encode` | `(data, window_size:, max_match:, min_match:) → [Token]` | Encode to token stream |
| `decode` | `(tokens, initial_buffer:) → String` | Decode token stream |
| `compress` | `(data, window_size:, max_match:, min_match:) → String` | Encode + serialise |
| `decompress` | `(binary_string) → String` | Deserialise + decode |

### Token

```ruby
CodingAdventures::LZ77::Token.new(offset, length, next_char)
```

### Parameters

| Parameter   | Default | Meaning |
|-------------|---------|---------|
| window_size | 4096    | Maximum lookback distance. Larger = better compression. |
| max_match   | 255     | Maximum match length. |
| min_match   | 3       | Minimum match length before emitting a backreference. |

## Development

```bash
bundle install
bundle exec rake test
```

Coverage target: 95%+ (currently 100%).
