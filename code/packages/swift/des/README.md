# DES (Swift)

Data Encryption Standard (DES) block cipher — FIPS 46-3 — implemented from scratch in Swift for educational purposes.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.

## What It Implements

- **DES-56** — 16-round Feistel network with 56-bit effective key
- **Key schedule** — PC-1/PC-2 selection, left-rotation schedule (SHIFTS), 16 round subkeys
- **Round function F** — E expansion, XOR with subkey, 8 S-box substitutions, P permutation
- **ECB mode** — Electronic Codebook with PKCS#7 padding
- **3DES/TDEA** — Triple-DES EDE per NIST SP 800-67

## Usage

```swift
import DES

let key   = [UInt8](hex: "133457799bbcdff1")
let plain = [UInt8](hex: "0123456789abcdef")

let ct = desEncryptBlock(plain, key: key)
let pt = desDecryptBlock(ct, key: key)

// ECB mode
let ciphertext = desECBEncrypt(Array("Hello world".utf8), key: key)
let plaintext  = desECBDecrypt(ciphertext, key: key)

// 3DES
let k1 = [UInt8](hex: "0123456789abcdef")
let k2 = [UInt8](hex: "23456789abcdef01")
let k3 = [UInt8](hex: "456789abcdef0123")
let ct3 = tdeaEncryptBlock(plain, k1: k1, k2: k2, k3: k3)
```
