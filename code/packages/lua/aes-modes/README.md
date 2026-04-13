# coding-adventures-aes-modes (Lua)

AES modes of operation: ECB, CBC, CTR, and GCM. Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## What This Package Does

AES operates on fixed 16-byte blocks. This package provides four modes of operation that extend AES to handle arbitrary-length messages:

| Mode | Security | Use Case |
|------|----------|----------|
| ECB  | INSECURE | Educational only — identical blocks produce identical ciphertext |
| CBC  | Legacy   | XOR-chains blocks; vulnerable to padding oracle attacks |
| CTR  | Modern   | Turns block cipher into stream cipher; parallelizable |
| GCM  | Best     | CTR + authentication tag; used in TLS 1.3 |

## Dependencies

- `coding-adventures-aes` — AES block cipher (encrypt_block / decrypt_block)

## Usage

```lua
local aes_modes = require("coding_adventures.aes_modes")

-- ECB (INSECURE — educational only)
local ct = aes_modes.ecb_encrypt(plaintext, key)
local pt = aes_modes.ecb_decrypt(ct, key)

-- CBC
local ct = aes_modes.cbc_encrypt(plaintext, key, iv_16)
local pt = aes_modes.cbc_decrypt(ct, key, iv_16)

-- CTR (no padding needed)
local ct = aes_modes.ctr_encrypt(plaintext, key, nonce_12)
local pt = aes_modes.ctr_decrypt(ct, key, nonce_12)

-- GCM (authenticated encryption)
local ct, tag = aes_modes.gcm_encrypt(plaintext, key, iv_12, aad)
local pt = aes_modes.gcm_decrypt(ct, key, iv_12, aad, tag)
```

## Running Tests

```bash
cd tests && busted . --verbose --pattern=test_
```
