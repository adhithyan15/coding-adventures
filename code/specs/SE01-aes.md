# SE01 — AES (Advanced Encryption Standard)

## Overview

AES is a symmetric block cipher adopted by NIST in 2001 (FIPS 197) to
replace DES. Designed by Joan Daemen and Vincent Rijmen (originally named
Rijndael), it operates on 128-bit blocks with key sizes of 128, 192, or
256 bits.

AES is the most widely deployed encryption algorithm in the world — used
in TLS/HTTPS, WiFi (WPA2/WPA3), disk encryption (BitLocker, FileVault,
LUKS), VPNs (IPsec, WireGuard), and virtually every secure protocol.

### Block Cipher vs Stream Cipher

AES encrypts exactly one 128-bit block. To encrypt arbitrary-length data,
you need a **mode of operation** (ECB, CBC, CTR, GCM). This package
implements the core block cipher; modes are in SE02.

## Algorithm

AES operates on a 4×4 matrix of bytes called the **state**.

### Key Schedule
- AES-128: 10 rounds, 11 round keys (176 bytes)
- AES-192: 12 rounds, 13 round keys (208 bytes)
- AES-256: 14 rounds, 15 round keys (240 bytes)

Key expansion uses:
- RotWord: circular left shift of 4-byte word
- SubWord: apply S-box to each byte
- XOR with round constant (Rcon)

### Encryption (per block)
```
1. AddRoundKey(state, round_key[0])        // XOR with first round key
2. For round = 1 to Nr-1:
   a. SubBytes(state)                       // S-box substitution
   b. ShiftRows(state)                      // cyclic row shifts
   c. MixColumns(state)                     // GF(2^8) column mixing
   d. AddRoundKey(state, round_key[round])
3. SubBytes(state)                          // final round (no MixColumns)
4. ShiftRows(state)
5. AddRoundKey(state, round_key[Nr])
```

### SubBytes
Each byte is replaced using a fixed 256-byte substitution table (S-box)
derived from the multiplicative inverse in GF(2^8) followed by an affine
transformation.

### ShiftRows
- Row 0: no shift
- Row 1: shift left 1
- Row 2: shift left 2
- Row 3: shift left 3

### MixColumns
Each column is multiplied by a fixed matrix in GF(2^8):
```
[2 3 1 1]   [s0]
[1 2 3 1] × [s1]
[1 1 2 3]   [s2]
[3 1 1 2]   [s3]
```

Multiplication in GF(2^8) uses the irreducible polynomial x^8 + x^4 + x^3 + x + 1.

### Decryption
Applies inverse operations in reverse order:
InvShiftRows, InvSubBytes, AddRoundKey, InvMixColumns

## Interface Contract

| Function | Signature | Description |
|----------|-----------|-------------|
| `aes_encrypt_block` | `(block: 16 bytes, key: 16/24/32 bytes) -> 16 bytes` | Encrypt one 128-bit block. |
| `aes_decrypt_block` | `(block: 16 bytes, key: 16/24/32 bytes) -> 16 bytes` | Decrypt one 128-bit block. |
| `expand_key` | `(key: 16/24/32 bytes) -> round_keys` | Key schedule expansion. |
| `SBOX` | constant | 256-byte substitution table. |
| `INV_SBOX` | constant | 256-byte inverse substitution table. |

### Key size determines rounds:
- 16 bytes (128-bit): 10 rounds
- 24 bytes (192-bit): 12 rounds
- 32 bytes (256-bit): 14 rounds

## Test Vectors (FIPS 197 Appendix B)

```
# AES-128
Key:       2b7e151628aed2a6abf7158809cf4f3c
Plaintext: 3243f6a8885a308d313198a2e0370734
Ciphertext: 3925841d02dc09fbdc118597196a0b32

# AES-256
Key:       603deb1015ca71be2b73aef0857d7781
           1f352c073b6108d72d9810a30914dff4
Plaintext: 6bc1bee22e409f96e93d7e117393172a
Ciphertext: f3eed1bdb5d2a03c064b5a7e3db181f8
```

## Design Notes

- **S-box generation:** The S-box can be generated at initialization or
  hardcoded as a constant. Generating demonstrates the GF(2^8) math;
  hardcoding is faster.
- **T-tables:** Production implementations often precompute 4 × 256
  lookup tables (T-tables) that combine SubBytes, ShiftRows, and
  MixColumns into a single table lookup per byte. Educational
  implementations should show the individual steps first.
- **Constant-time:** Production AES must avoid timing side channels
  (cache-timing attacks). Educational implementations may note this
  without implementing it.

## Package Matrix

Same 9 languages, in `aes/` directories.

**Dependencies:** None for core block cipher. GF(2^8) arithmetic is
self-contained (may reuse `gf256` package if available).
