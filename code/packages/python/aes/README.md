# coding-adventures-aes

AES (Advanced Encryption Standard) block cipher — FIPS 197 — implemented from scratch, using GF(2^8) arithmetic to show the mathematical foundations.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.

## What It Implements

- **AES-128, AES-192, AES-256** — all three key sizes, 10/12/14 rounds respectively
- **Key schedule** — word-based expansion via RotWord, SubWord, Rcon
- **SubBytes / InvSubBytes** — S-box generated from GF(2^8) multiplicative inverse + affine transform using `GF256Field(0x11B)`
- **ShiftRows / InvShiftRows** — cyclic row shifts for inter-column diffusion
- **MixColumns / InvMixColumns** — column mixing via GF(2^8) matrix multiply
- **SBOX / INV_SBOX** — exported 256-byte constants for inspection

## Installation

```bash
pip install coding-adventures-aes
```

## Usage

```python
from coding_adventures_aes import aes_encrypt_block, aes_decrypt_block, SBOX

# AES-128
key   = bytes.fromhex("2b7e151628aed2a6abf7158809cf4f3c")
plain = bytes.fromhex("3243f6a8885a308d313198a2e0370734")
ct    = aes_encrypt_block(plain, key)   # → 3925841d02dc09fbdc118597196a0b32
pt    = aes_decrypt_block(ct, key)      # → original plaintext

# AES-256
key256 = bytes.fromhex("603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4")
ct256  = aes_encrypt_block(plain, key256)

# Inspect the S-box (generated from GF(2^8) arithmetic with polynomial 0x11B)
print(f"SBOX[0x00] = {SBOX[0x00]:#04x}")  # → 0x63
```

## GF(2^8) Connection

AES arithmetic uses a different field polynomial than Reed-Solomon. The `gf256` package defaults to `0x11D` (Reed-Solomon), but AES needs `0x11B` (`x^8 + x^4 + x^3 + x + 1`). This package uses `GF256Field(0x11B)` to get the right field instance:

```python
from coding_adventures_gf256 import GF256Field
aes_field = GF256Field(0x11B)
# Used to generate the S-box and compute MixColumns
```

This is why a parameterizable GF256 field was needed — and why AES was the motivating use case.
