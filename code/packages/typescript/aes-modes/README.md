# @coding-adventures/aes-modes

AES modes of operation — ECB, CBC, CTR, GCM — implemented from scratch for educational purposes.

## Overview

AES operates on fixed 128-bit (16-byte) blocks. To encrypt arbitrary-length messages, you need a **mode of operation** that chains multiple block cipher calls. This package implements four modes with increasing security:

| Mode | Security | Use Case |
|------|----------|----------|
| ECB  | **BROKEN** | Education only — identical blocks leak patterns |
| CBC  | Legacy | Was standard in TLS; vulnerable to padding oracle attacks |
| CTR  | Modern | Stream cipher mode — parallelizable, no padding |
| GCM  | Modern + Auth | CTR + GHASH authentication — gold standard for TLS 1.3 |

## Installation

```bash
npm install
```

Depends on `@coding-adventures/aes` (the AES block cipher).

## Usage

```typescript
import { ecbEncrypt, ecbDecrypt, cbcEncrypt, cbcDecrypt,
         ctrEncrypt, ctrDecrypt, gcmEncrypt, gcmDecrypt,
         fromHex } from "@coding-adventures/aes-modes";

const key = fromHex("2b7e151628aed2a6abf7158809cf4f3c");
const plaintext = new TextEncoder().encode("Hello, AES modes!");

// ECB (INSECURE — educational only)
const ecbCt = ecbEncrypt(plaintext, key);
const ecbPt = ecbDecrypt(ecbCt, key);

// CBC
const iv = fromHex("000102030405060708090a0b0c0d0e0f");
const cbcCt = cbcEncrypt(plaintext, key, iv);
const cbcPt = cbcDecrypt(cbcCt, key, iv);

// CTR
const nonce = fromHex("f0f1f2f3f4f5f6f7f8f9fafb");
const ctrCt = ctrEncrypt(plaintext, key, nonce);
const ctrPt = ctrDecrypt(ctrCt, key, nonce);

// GCM (authenticated encryption)
const aad = new Uint8Array(0);
const { ciphertext, tag } = gcmEncrypt(plaintext, key, iv12, aad);
const gcmPt = gcmDecrypt(ciphertext, key, iv12, aad, tag);
```

## Testing

```bash
npm test              # run tests
npm run test:coverage # run with coverage
```

Uses NIST SP 800-38A test vectors for ECB/CBC/CTR and NIST GCM specification vectors for GCM.

## Part of coding-adventures

An educational computing stack built from logic gates through compilers.
