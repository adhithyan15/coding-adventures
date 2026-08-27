# Vigenere Cipher (Lua)

## CR03 conformance

This implementation follows [CR03](../../../specs/CR03-vigenere-cipher.md): keys and cipher transforms use ASCII letters only, analysis ignores non-ASCII letters, and non-ASCII Unicode scalars pass through without advancing the key. Analysis accepts at most 8,192 Unicode scalars and key-length bounds up to 40, uses the smallest candidate within 90% of the best index-of-coincidence score, preserves the exact requested recovered-key length, and resolves score ties toward the smallest shift.

A Lua implementation of the Vigenere polyalphabetic substitution cipher with full cryptanalysis support (key length detection via Index of Coincidence and key recovery via chi-squared analysis).

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## Installation

```lua
luarocks install coding-adventures-vigenere-cipher
```

## Usage

```lua
local vigenere = require("coding_adventures.vigenere_cipher")

-- Encrypt
local ct = vigenere.encrypt("ATTACKATDAWN", "LEMON")  -- "LXFOPVEFRNHR"

-- Decrypt
local pt = vigenere.decrypt("LXFOPVEFRNHR", "LEMON")  -- "ATTACKATDAWN"

-- Cryptanalysis (requires 200+ character ciphertext)
local key_len = vigenere.find_key_length(long_ciphertext)
local key = vigenere.find_key(long_ciphertext, key_len)
local recovered_key, plaintext = vigenere.break_cipher(long_ciphertext)
```

## API

| Function | Description |
|----------|-------------|
| `encrypt(plaintext, key)` | Encrypt using Vigenere cipher |
| `decrypt(ciphertext, key)` | Decrypt using Vigenere cipher |
| `find_key_length(ciphertext, max_length?)` | Estimate key length via IC analysis |
| `find_key(ciphertext, key_length)` | Find key via chi-squared analysis |
| `break_cipher(ciphertext)` | Full automatic break (returns key, plaintext) |

## Testing

```sh
cd tests && busted . --verbose --pattern=test_
```
