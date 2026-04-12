# @coding-adventures/aes

AES (Advanced Encryption Standard) block cipher — FIPS 197. Supports AES-128, AES-192, and AES-256.

## What Is AES?

AES is the most widely deployed symmetric encryption algorithm in the world. Published by NIST in 2001 as FIPS 197, it replaced DES and is used in TLS/HTTPS, WPA2/WPA3 WiFi, disk encryption (BitLocker, LUKS, FileVault), VPNs, and virtually every secure protocol.

Designed by Joan Daemen and Vincent Rijmen (Rijndael), AES is a Substitution-Permutation Network (SPN) — a fundamentally different structure from DES's Feistel network. All 16 bytes of the state are transformed on every round.

## Usage

```typescript
import { aesEncryptBlock, aesDecryptBlock, fromHex, toHex } from "@coding-adventures/aes";

// AES-128
const key128 = fromHex("2b7e151628aed2a6abf7158809cf4f3c");
const plain  = fromHex("3243f6a8885a308d313198a2e0370734");
const cipher = aesEncryptBlock(plain, key128);
console.log(toHex(cipher)); // → "3925841d02dc09fbdc118597196a0b32"

// AES-256
const key256 = fromHex("603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4");
const ct256  = aesEncryptBlock(plain, key256);
```

## API

| Function | Description |
|---|---|
| `aesEncryptBlock(block, key)` | Encrypt one 16-byte block (AES-128/192/256) |
| `aesDecryptBlock(block, key)` | Decrypt one 16-byte block |
| `expandKey(key)` | Expand key into round keys |
| `SBOX` | 256-entry AES S-box |
| `INV_SBOX` | 256-entry inverse S-box |
| `toHex(bytes)` | Uint8Array → hex string |
| `fromHex(hex)` | hex string → Uint8Array |

## How It Works

### Four Core Operations

**SubBytes** — replaces each byte with its GF(2^8) multiplicative inverse, then applies an affine transformation. This is the only non-linear step.

**ShiftRows** — cyclically shifts row i left by i positions. Ensures cross-column diffusion after MixColumns.

**MixColumns** — treats each 4-byte column as a polynomial in GF(2^8) and multiplies by the AES MixColumns matrix. Provides row diffusion.

**AddRoundKey** — XORs the state with the current round key. Since XOR is its own inverse, this step is identical for encryption and decryption.

### GF(2^8) Arithmetic

AES arithmetic uses GF(2^8) with the irreducible polynomial:
  `p(x) = x^8 + x^4 + x^3 + x + 1  =  0x11B`

This package uses `@coding-adventures/gf256`'s `createField(0x11B)` for all field operations.

## Key Sizes

| Key | Rounds | Round Keys |
|---|---|---|
| 128 bits (16 bytes) | 10 | 11 |
| 192 bits (24 bytes) | 12 | 13 |
| 256 bits (32 bytes) | 14 | 15 |

## Implementation Notes

- Follows FIPS 197 exactly
- Block size is always 128 bits (16 bytes) regardless of key size
- S-box is computed at module load time from GF(2^8) inverses + affine transform
- Decryption uses distinct inverse operations (InvSubBytes, InvShiftRows, InvMixColumns)

## References

- FIPS 197: https://csrc.nist.gov/publications/detail/fips/197/final
- FIPS 197 Appendix B: step-by-step AES-128 worked example
- FIPS 197 Appendix C: AES-128/192/256 test vectors
