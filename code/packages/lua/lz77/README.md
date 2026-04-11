# lz77 — LZ77 Lossless Compression Algorithm (Lua)

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

```lua
local lz77 = require("coding_adventures.lz77")

-- One-shot compression / decompression (string API)
local data = "hello hello hello world"
local compressed = lz77.compress(data)
local original   = lz77.decompress(compressed)

-- Token-level API (byte array)
local bytes = {data:byte(1, #data)}
local tokens = lz77.encode(bytes)
local decoded = lz77.decode_to_string(tokens)

-- String-level encode/decode
local tokens2 = lz77.encode_string(data)
local result   = lz77.decode_to_string(tokens2)
```

## API

| Function | Description |
|----------|-------------|
| `encode(data, window_size, max_match, min_match)` | Encode byte array to tokens |
| `encode_string(str, ...)` | String wrapper for encode |
| `decode(tokens, initial_buffer)` | Decode tokens to byte array |
| `decode_to_string(tokens, initial_buffer)` | Decode tokens to string |
| `compress(str, ...)` | Encode + serialise to binary string |
| `decompress(data)` | Deserialise + decode to string |
| `serialise_tokens(tokens)` | Serialise to binary string |
| `deserialise_tokens(data)` | Deserialise from binary string |
| `token(offset, length, next_char)` | Create a token table |

### Parameters

| Parameter   | Default | Meaning |
|-------------|---------|---------|
| window_size | 4096    | Maximum lookback distance. |
| max_match   | 255     | Maximum match length. |
| min_match   | 3       | Minimum match length for backreference. |

## Development

```bash
cd tests && LUA_PATH="../src/?.lua;../src/?/init.lua;;" busted . --verbose --pattern=test_
```

30 successes / 0 failures.
