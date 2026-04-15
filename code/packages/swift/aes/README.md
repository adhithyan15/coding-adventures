# AES (Swift)

AES (Advanced Encryption Standard) block cipher — FIPS 197 — implemented from scratch in Swift for educational purposes.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.

## What It Implements

- **AES-128, AES-192, AES-256** — all three key sizes, 10/12/14 rounds
- **Key schedule** — word-based expansion via RotWord, SubWord, Rcon
- **SubBytes / InvSubBytes** — S-box from GF(2^8) inverse + affine transform (polynomial 0x11B)
- **ShiftRows / InvShiftRows**, **MixColumns / InvMixColumns**
- **sbox / invSbox** — exported 256-element constant arrays

## Usage

```swift
import AES

let key   = h("2b7e151628aed2a6abf7158809cf4f3c")
let plain = h("3243f6a8885a308d313198a2e0370734")

let ct = aesEncryptBlock(plain, key: key)
let pt = aesDecryptBlock(ct, key: key)
```
