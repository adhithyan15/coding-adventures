# coding-adventures-aes (Lua)

AES block cipher (FIPS 197). Supports AES-128, AES-192, and AES-256.

## Usage

```lua
local aes = require("coding_adventures.aes")

local function h(hex)
    return (hex:gsub("..", function(x) return string.char(tonumber(x, 16)) end))
end

local key   = h("2b7e151628aed2a6abf7158809cf4f3c")
local plain = h("3243f6a8885a308d313198a2e0370734")
local ct = aes.aes_encrypt_block(plain, key)
local pt = aes.aes_decrypt_block(ct, key)
```

## Running Tests

```bash
cd tests && busted . --verbose --pattern=test_
```
