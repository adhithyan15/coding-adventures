# lzss — LZSS Lossless Compression Algorithm (Lua)

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

## Usage

```lua
local lzss = require("coding_adventures.lzss")

-- One-shot compression / decompression
local data       = "hello hello hello world"
local compressed = lzss.compress(data)
local original   = lzss.decompress(compressed)  -- "hello hello hello world"

-- Token-level API
local tokens = lzss.encode_string(data)
local result = lzss.decode_to_string(tokens)

-- Custom parameters
local tokens2 = lzss.encode_string(data, 2048, 128, 3)
```

## API

| Function | Description |
|----------|-------------|
| `encode(data, ...)` | Encode 1-indexed byte array to token list |
| `encode_string(str, ...)` | Convenience wrapper for string input |
| `decode(tokens, orig_len)` | Decode tokens to byte array |
| `decode_to_string(tokens, orig_len)` | Decode tokens to Lua string |
| `compress(str, ...)` | Encode + serialise to CMP02 wire format |
| `decompress(data)` | Deserialise + decode |
| `serialise_tokens(tokens, orig_len)` | Serialise token list to binary |
| `deserialise_tokens(data)` | Deserialise binary; returns `(tokens, orig_len)` |
| `literal(byte)` | Create a Literal token |
| `match(offset, length)` | Create a Match token |

### Tokens

- `{kind="literal", byte=integer}` — a raw byte (0–255).
- `{kind="match", offset=integer, length=integer}` — back-reference.

### Parameters

| Parameter   | Default | Meaning |
|-------------|---------|---------|
| window_size | 4096    | Maximum lookback distance. |
| max_match   | 255     | Maximum match length. |
| min_match   | 3       | Minimum match length for a Match token. |

## Development

```bash
cd tests && LUA_PATH="../src/?.lua;../src/?/init.lua;;" busted . --verbose --pattern=test_
```
