# ChaCha20-Poly1305 (Lua)

ChaCha20-Poly1305 authenticated encryption (RFC 8439) implemented in pure Lua 5.4.

## What is ChaCha20-Poly1305?

ChaCha20-Poly1305 is an authenticated encryption scheme combining:

- **ChaCha20**: a stream cipher using only Add, Rotate, XOR (ARX) operations
- **Poly1305**: a one-time message authentication code (MAC)

Together they provide both confidentiality (encryption) and integrity (authentication). Used in TLS 1.3, WireGuard, SSH, and Chrome/Android.

## Usage

```lua
local cc = require("coding_adventures.chacha20_poly1305")

-- ChaCha20 stream cipher
local ct = cc.chacha20_encrypt(plaintext, key_32, nonce_12, counter)

-- Poly1305 MAC
local tag = cc.poly1305_mac(message, key_32)

-- AEAD encrypt
local ciphertext, tag = cc.aead_encrypt(plaintext, key_32, nonce_12, aad)

-- AEAD decrypt (returns nil + error on auth failure)
local plaintext, err = cc.aead_decrypt(ciphertext, key_32, nonce_12, aad, tag)
```

## API

| Function | Parameters | Returns |
|----------|-----------|---------|
| `chacha20_encrypt` | `(data, key_32, nonce_12, counter)` | ciphertext (same length) |
| `poly1305_mac` | `(message, key_32)` | 16-byte tag |
| `aead_encrypt` | `(plaintext, key_32, nonce_12, aad)` | ciphertext, tag |
| `aead_decrypt` | `(ciphertext, key_32, nonce_12, aad, tag)` | plaintext or nil, error |

## Running Tests

```sh
cd tests && busted . --verbose --pattern=test_
```

## Dependencies

None. Pure Lua 5.4 (uses native bitwise operators).
