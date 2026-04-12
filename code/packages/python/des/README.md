# coding-adventures-des

DES (Data Encryption Standard) and 3DES block cipher — FIPS 46-3 / SP 800-67 — implemented from scratch for educational purposes.

**Do not use DES for anything new.** It was withdrawn by NIST in 2005. A 56-bit key can be exhausted in under 24 hours on consumer hardware. This package exists to understand cryptographic history: Feistel networks, S-box design, and why key size matters.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.

## What It Implements

- **DES block cipher** — 64-bit block, 56-bit effective key, 16 Feistel rounds
- **Key schedule** — PC-1/PC-2 permutations, left-rotation expansion to 16 × 48-bit subkeys
- **ECB mode** — with PKCS#7 padding (educational; ECB is insecure for real use)
- **3DES (TDEA)** — EDE (Encrypt-Decrypt-Encrypt) with three independent keys

## Installation

```bash
pip install coding-adventures-des
```

## Usage

```python
from coding_adventures_des import (
    des_encrypt_block, des_decrypt_block,
    des_ecb_encrypt, des_ecb_decrypt,
    tdea_encrypt_block, tdea_decrypt_block,
    expand_key,
)

# Single block (8 bytes in, 8 bytes out)
key   = bytes.fromhex("0133457799BBCDFF")
plain = bytes.fromhex("0123456789ABCDEF")
ct    = des_encrypt_block(plain, key)    # → 85E813540F0AB405
pt    = des_decrypt_block(ct, key)       # → 0123456789ABCDEF

# Variable-length ECB (PKCS#7 padding)
ct = des_ecb_encrypt(b"Hello, World!", key)
pt = des_ecb_decrypt(ct, key)  # → b"Hello, World!"

# 3DES EDE
k1 = bytes.fromhex("0123456789ABCDEF")
k2 = bytes.fromhex("23456789ABCDEF01")
k3 = bytes.fromhex("456789ABCDEF0123")
ct = tdea_encrypt_block(plain, k1, k2, k3)
pt = tdea_decrypt_block(ct, k1, k2, k3)

# Inspect the key schedule
subkeys = expand_key(key)  # list of 16 × 6-byte subkeys
```

## Why Study DES?

DES introduced two ideas that define modern symmetric cryptography:

1. **Feistel networks** — encrypt and decrypt with the same circuit; just reverse the subkey order. AES, Blowfish, Twofish all descend from this insight.
2. **S-box based non-linearity** — the only part of DES that resists algebraic attacks. DES's S-boxes were secretly hardened against differential cryptanalysis by the NSA — a technique not publicly known until 1990.

Understanding where DES failed (56-bit key = 2^56 ≈ 72 quadrillion brute-force attempts, now feasible) makes AES's 128-bit minimum (2^128 — infeasible for the foreseeable future) legible.
