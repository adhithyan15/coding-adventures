# coding-adventures-des (Lua)

DES and Triple DES (TDEA) block cipher — FIPS 46-3 / SP 800-67.

**Warning:** DES is cryptographically broken. Use for education only.

## Usage

```lua
local des = require("coding_adventures.des")

local function h(hex)
    return (hex:gsub("..", function(x) return string.char(tonumber(x, 16)) end))
end

-- Single-block DES
local key   = h("133457799BBCDFF1")
local plain = h("0123456789ABCDEF")
local ct = des.des_encrypt_block(plain, key)
-- Decrypt
local pt = des.des_decrypt_block(ct, key)

-- ECB mode
local ct2 = des.des_ecb_encrypt("Hello, World!", key)
local pt2 = des.des_ecb_decrypt(ct2, key)

-- Triple DES
local k1 = h("0123456789ABCDEF")
local k2 = h("23456789ABCDEF01")
local k3 = h("456789ABCDEF0123")
local ct3 = des.tdea_encrypt_block(plain, k1, k2, k3)
```

## Running Tests

```bash
cd tests && busted . --verbose --pattern=test_
```
