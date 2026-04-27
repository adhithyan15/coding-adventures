# @coding-adventures/des

DES (Data Encryption Standard) and Triple DES block cipher — FIPS 46-3 / NIST SP 800-67.

## What Is DES?

DES was published by NIST in 1977 as the world's first openly standardized symmetric encryption algorithm. It was designed by IBM and hardened by the NSA using a 56-bit key (8 parity bits in the 64-bit key are discarded). A 56-bit key is completely broken by modern hardware — exhaustive search takes under 24 hours on consumer GPUs.

Despite being broken, DES is foundational to cryptography education:

- **Feistel networks** — the structural pattern shared by Blowfish, Twofish, CAST5, and many others
- **S-boxes** — the non-linear heart that resists differential and linear cryptanalysis
- **Key schedules** — how a master key expands into per-round subkeys
- **3DES** — the backward-compatible extension using Encrypt-Decrypt-Encrypt (EDE) to extend DES to ~112-bit effective security

## Usage

```typescript
import { desEncryptBlock, desDecryptBlock, fromHex, toHex } from "@coding-adventures/des";

const key   = fromHex("133457799BBCDFF1");
const plain = fromHex("0123456789ABCDEF");

const cipher = desEncryptBlock(plain, key);
console.log(toHex(cipher)); // → "85e813540f0ab405"

const recovered = desDecryptBlock(cipher, key);
console.log(toHex(recovered)); // → "0123456789abcdef"
```

### ECB Mode

```typescript
import { desEcbEncrypt, desEcbDecrypt } from "@coding-adventures/des";

const key   = fromHex("133457799BBCDFF1");
const plain = new TextEncoder().encode("Hello, DES!");

const ct = desEcbEncrypt(plain, key);
const pt = desEcbDecrypt(ct, key);
// pt equals plain
```

### Triple DES (3DES / TDEA)

```typescript
import { tdeaEncryptBlock, tdeaDecryptBlock } from "@coding-adventures/des";

const k1 = fromHex("0123456789ABCDEF");
const k2 = fromHex("23456789ABCDEF01");
const k3 = fromHex("456789ABCDEF0123");
const plain = fromHex("6BC1BEE22E409F96");

const cipher = tdeaEncryptBlock(plain, k1, k2, k3);
// → "3B6423D418DEFC23"
```

## API

| Function | Description |
|---|---|
| `expandKey(key)` | Derive 16 round subkeys from an 8-byte key |
| `desEncryptBlock(block, key)` | Encrypt one 8-byte block |
| `desDecryptBlock(block, key)` | Decrypt one 8-byte block |
| `desEcbEncrypt(plain, key)` | ECB-mode encryption with PKCS#7 padding |
| `desEcbDecrypt(cipher, key)` | ECB-mode decryption (removes PKCS#7 padding) |
| `tdeaEncryptBlock(block, k1, k2, k3)` | 3DES EDE encrypt |
| `tdeaDecryptBlock(block, k1, k2, k3)` | 3DES EDE decrypt |
| `toHex(bytes)` | Uint8Array → hex string |
| `fromHex(hex)` | hex string → Uint8Array |

## Security Warning

**Do not use DES or 3DES to protect real data.**

- DES (56-bit key) has been broken since 1998 (EFF DES Cracker).
- 3DES (~112-bit effective security) was deprecated by NIST in 2017 and disallowed in 2023 (SWEET32 attack on 64-bit block ciphers).
- Use AES-128 or AES-256 for new applications.

## How It Fits in the Stack

This package implements the raw DES block cipher. For production use cases:

- Block cipher modes (CBC, CTR, GCM) — see the modes-of-operation package
- Authenticated encryption — see AES-GCM
- Key derivation — see PBKDF2, scrypt

## Implementation Notes

- Follows FIPS 46-3 exactly, working with bit arrays for clarity
- 3DES uses EDE ordering: `E_K1(D_K2(E_K3(P)))`
- ECB mode uses PKCS#7 padding (always adds a padding block when data is exactly block-aligned)
- When K1=K2=K3, 3DES reduces to single DES (backward compatibility)

## References

- FIPS 46-3 (withdrawn 2005): https://csrc.nist.gov/publications/detail/fips/46/3/final
- NIST SP 800-67: Triple Data Encryption Algorithm
- NIST SP 800-20: Modes of Operation Validation System
