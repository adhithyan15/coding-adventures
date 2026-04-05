# SHA-512 (Swift)

Pure Swift implementation of the SHA-512 cryptographic hash function (FIPS 180-4).

## Overview

SHA-512 is the 64-bit sibling of SHA-256 in the SHA-2 family. It produces a 512-bit (64-byte) digest using 8 x 64-bit state words and 80 rounds of compression. On 64-bit platforms, SHA-512 is often faster than SHA-256 because it processes 128-byte blocks using native 64-bit arithmetic.

## Usage

```swift
import SHA512

// One-shot hash
let digest = sha512(Data("hello".utf8))      // 64-byte Data
let hex = sha512Hex(Data("hello".utf8))       // 128-char hex string

// Streaming hash
var hasher = SHA512Hasher()
hasher.update(Data("ab".utf8))
hasher.update(Data("c".utf8))
let result = hasher.hexDigest()               // same as sha512Hex("abc")
```

## API

| Function / Type | Description |
|----------------|-------------|
| `sha512(Data) -> Data` | Returns 64-byte digest |
| `sha512Hex(Data) -> String` | Returns 128-character lowercase hex string |
| `SHA512Hasher` | Streaming hasher with `update`, `digest`, `hexDigest`, `copy` |

## Part of coding-adventures

An educational computing stack built from logic gates up through interpreters and compilers.
