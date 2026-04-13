# coding-adventures-deflate (Lua)

**CMP05 — DEFLATE lossless compression (1996)**

## Usage

```lua
local deflate = require("coding_adventures.deflate")

local data = "hello hello hello world"
local compressed = deflate.compress(data)
local original = deflate.decompress(compressed)
assert(original == data)
```

## Wire Format

```
[4B] original_length    big-endian uint32
[2B] ll_entry_count     big-endian uint16
[2B] dist_entry_count   big-endian uint16
[ll_entry_count × 3B]   (symbol uint16 BE, code_length uint8)
[dist_entry_count × 3B] same format
[remaining bytes]       LSB-first packed bit stream
```
