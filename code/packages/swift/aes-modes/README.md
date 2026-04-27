# AESModes

AES modes of operation — ECB, CBC, CTR, GCM — implemented from scratch in Swift for educational purposes.

## Overview

AES operates on fixed 128-bit blocks. To encrypt arbitrary-length messages, you need a **mode of operation**. This package implements four modes:

| Mode | Security | Properties |
|------|----------|------------|
| ECB  | **BROKEN** | Identical blocks leak patterns |
| CBC  | Legacy | Padding oracle vulnerable |
| CTR  | Modern | Stream cipher, parallelizable |
| GCM  | Modern + Auth | CTR + GHASH, gold standard |

## Usage

```swift
import AESModes

let key = fromHex("2b7e151628aed2a6abf7158809cf4f3c")
let plaintext = Array("Hello, AES modes!".utf8)

// ECB (INSECURE)
let ct = AESModes.ecbEncrypt(plaintext, key: key)

// CBC
let iv = fromHex("000102030405060708090a0b0c0d0e0f")
let ct = try AESModes.cbcEncrypt(plaintext, key: key, iv: iv)

// CTR
let nonce = fromHex("f0f1f2f3f4f5f6f7f8f9fafb")
let ct = try AESModes.ctrEncrypt(plaintext, key: key, nonce: nonce)

// GCM
let (ct, tag) = try AESModes.gcmEncrypt(plaintext, key: key, iv: nonce)
```

## Testing

```bash
swift test
```

Uses NIST SP 800-38A and GCM specification test vectors.

## Part of coding-adventures

An educational computing stack built from logic gates through compilers.
