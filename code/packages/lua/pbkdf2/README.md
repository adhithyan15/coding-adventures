# coding-adventures-pbkdf2 (Lua)

PBKDF2 (Password-Based Key Derivation Function 2) — RFC 8018 — implemented from scratch in Lua 5.4.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.

## Usage

```lua
local pbkdf2 = require("coding_adventures.pbkdf2")

local dk = pbkdf2.pbkdf2_hmac_sha256(
    "correct horse battery staple",
    "\xde\xad\xbe\xef\xde\xad\xbe\xef\xde\xad\xbe\xef\xde\xad\xbe\xef",
    600000,   -- OWASP 2023 minimum for SHA-256
    32
)
```

## API

| Function                     | PRF         | Returns          |
|------------------------------|-------------|------------------|
| `pbkdf2_hmac_sha1`           | HMAC-SHA1   | byte string      |
| `pbkdf2_hmac_sha256`         | HMAC-SHA256 | byte string      |
| `pbkdf2_hmac_sha512`         | HMAC-SHA512 | byte string      |
| `pbkdf2_hmac_sha1_hex`       | HMAC-SHA1   | hex string       |
| `pbkdf2_hmac_sha256_hex`     | HMAC-SHA256 | hex string       |
| `pbkdf2_hmac_sha512_hex`     | HMAC-SHA512 | hex string       |

## Stack Position

KD01. Depends on `coding-adventures-hmac` (HF05).
