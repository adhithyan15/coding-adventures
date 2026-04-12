# LZ78 — Lossless Compression Algorithm (Lua)

Lua implementation of the LZ78 compression algorithm (Lempel & Ziv, 1978),
part of the CMP series in coding-adventures.

## Usage

```lua
local lz78 = require("coding_adventures.lz78")

-- One-shot compress/decompress
local compressed = lz78.compress("hello hello hello")
local original   = lz78.decompress(compressed)
-- original == "hello hello hello"

-- Token-level API
local tokens = lz78.encode("AABCBBABC")
local data   = lz78.decode(tokens, 9)
-- data == "AABCBBABC"
```

## TrieCursor

The `TrieCursor` object is exported for reuse in streaming dictionary algorithms
like LZW (CMP03):

```lua
local cursor = lz78.TrieCursor.new()
cursor:insert(65, 1)          -- add root→'A'→id=1
if cursor:step(65) then       -- true: found 'A'
  print(cursor:dict_id())     -- 1
end
cursor:reset()                -- back to root
```

## In the Series

| Spec  | Algorithm  | Year | Key Concept                          |
|-------|-----------|------|--------------------------------------|
| CMP00 | LZ77      | 1977 | Sliding-window backreferences        |
| CMP01 | **LZ78**  | 1978 | Explicit dictionary (trie)           |
| CMP03 | LZW       | 1984 | LZ78 + pre-initialised alphabet; GIF |

## Development

```bash
cd tests && LUA_PATH="../src/?.lua;../src/?/init.lua;;" busted . --verbose --pattern=test_
```
