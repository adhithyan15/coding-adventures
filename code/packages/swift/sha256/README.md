# SHA256 (Swift)

Pure Swift implementation of the SHA-256 cryptographic hash function (FIPS 180-4).

## Overview

SHA-256 is a member of the SHA-2 family that produces a 256-bit (32-byte) digest. It uses the Merkle-Damgard construction with 8 x 32-bit state words and 64 compression rounds per block. This implementation uses Swift's `UInt32` with `&+` for wrapping arithmetic and provides both one-shot and streaming APIs.

## Installation

Add to your `Package.swift`:

```swift
dependencies: [
    .package(path: "../sha256"),
]
```

## Usage

### One-shot API

```swift
import SHA256
import Foundation

// Hex digest (64-character lowercase string)
let hex = sha256Hex(Data("hello".utf8))
// "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"

// Raw digest (32-byte Data)
let raw = sha256(Data("hello".utf8))
```

### Streaming API

```swift
import SHA256
import Foundation

var hasher = SHA256Hasher()
hasher.update(Data("hello ".utf8))
hasher.update(Data("world".utf8))
print(hasher.hexDigest())  // same as sha256Hex("hello world")

// Non-destructive: can call digest multiple times
let d1 = hasher.hexDigest()
let d2 = hasher.hexDigest()
assert(d1 == d2)

// Copy for branching
var branch = hasher.copy()
branch.update(Data("!".utf8))
// branch and hasher now have different states
```

## API Reference

| Function | Returns | Description |
|----------|---------|-------------|
| `sha256(Data)` | `Data` | Raw 32-byte digest |
| `sha256Hex(Data)` | `String` | 64-char lowercase hex digest |
| `SHA256Hasher()` | struct | Create streaming hasher |
| `.update(Data)` | void | Feed bytes |
| `.digest()` | `Data` | Get 32-byte digest (non-destructive) |
| `.hexDigest()` | `String` | Get hex digest (non-destructive) |
| `.copy()` | `SHA256Hasher` | Deep copy for branching |

## Part of coding-adventures

An educational computing stack built from logic gates up through interpreters and compilers. This package implements HF03 (SHA-256) from the hash functions layer.
