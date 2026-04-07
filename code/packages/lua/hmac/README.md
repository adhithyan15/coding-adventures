# coding-adventures-hmac (Lua)

HMAC (Hash-based Message Authentication Code) — RFC 2104 / FIPS 198-1 — implemented from scratch in Lua.

## What Is HMAC?

HMAC takes a secret key and a message and produces a fixed-size authentication tag proving both
**message integrity** and **authenticity**. Used in TLS, JWT, WPA2, TOTP, and AWS Signature V4.

## API

```lua
local hmac = require("coding_adventures.hmac")

local key = string.rep("\x0b", 20)

-- Hex strings (most common use case)
hmac.hmac_sha256_hex(key, "Hi There")
-- "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"

-- Raw byte tables
local tag = hmac.hmac_sha256(key, "Hi There")  -- table of 32 integers (0-255)

-- All four variants
hmac.hmac_md5(key, msg)    -- 16-byte table
hmac.hmac_sha1(key, msg)   -- 20-byte table
hmac.hmac_sha256(key, msg) -- 32-byte table
hmac.hmac_sha512(key, msg) -- 64-byte table

-- Generic function (pass your own hash fn)
local tag2 = hmac.hmac(sha256_m.sha256, 64, key, "msg")
```

## Dependencies

- `coding-adventures-md5`
- `coding-adventures-sha1`
- `coding-adventures-sha256`
- `coding-adventures-sha512`

## How It Fits

```
md5 / sha1 / sha256 / sha512
         ↓
       hmac  ← you are here
         ↓
    pbkdf2 / hkdf  (next)
```
