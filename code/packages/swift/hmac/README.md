# HMAC (Swift)

HMAC (Hash-based Message Authentication Code) — RFC 2104 / FIPS 198-1 — implemented from scratch in Swift.

## What Is HMAC?

HMAC takes a secret key and a message and produces a fixed-size authentication tag proving both
**message integrity** and **authenticity**. Used in TLS, JWT, WPA2, TOTP, and AWS Signature V4.

## API

```swift
import HMAC
import Foundation

let key = Data(repeating: 0x0b, count: 20)
let msg = Data("Hi There".utf8)

// Hex strings
hmacSHA256Hex(key: key, message: msg)
// "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"

// Raw Data
let tag: Data = hmacSHA256(key: key, message: msg)  // 32 bytes

// All four variants
hmacMD5(key: key, message: msg)    // 16 bytes
hmacSHA1(key: key, message: msg)   // 20 bytes
hmacSHA256(key: key, message: msg) // 32 bytes
hmacSHA512(key: key, message: msg) // 64 bytes

// Generic function
let tag2 = hmac(hashFn: sha256, blockSize: 64, key: key, message: msg)
```

## Dependencies

- `MD5` (local package)
- `SHA1` (local package)
- `SHA256` (local package)
- `SHA512` (local package)

## How It Fits

```
MD5 / SHA1 / SHA256 / SHA512
         ↓
       HMAC  ← you are here
         ↓
    PBKDF2 / HKDF  (next)
```
