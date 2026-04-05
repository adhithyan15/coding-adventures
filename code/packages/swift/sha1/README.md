# SHA-1 (Swift)

Pure Swift implementation of the SHA-1 cryptographic hash function (FIPS 180-4) with no external dependencies.

## What It Does

SHA-1 takes any sequence of bytes and produces a fixed-size 20-byte (160-bit) digest. The same input always produces the same digest. This implementation provides both one-shot and streaming APIs.

SHA-1 is weakened (SHAttered attack, 2017) but remains safe for UUID v5 and Git. For new security applications, use SHA-256 or SHA-3.

## Where It Fits

```
Application / UUID v5
      |
      v
  sha1(data) --> 20-byte digest
      |
      v
  [This package -- no dependencies]
```

## API

### One-Shot

```swift
import SHA1

// Returns 20-byte Data
let digest = sha1(Data("abc".utf8))

// Returns 40-character hex string
let hex = sha1Hex(Data("abc".utf8))
// "a9993e364706816aba3e25717850c26c9cd0d89d"
```

### Streaming

```swift
var hasher = SHA1Hasher()
hasher.update(Data("ab".utf8))
hasher.update(Data("c".utf8))
hasher.hexDigest()  // "a9993e364706816aba3e25717850c26c9cd0d89d"

// Non-destructive: digest() can be called multiple times
// copy() creates an independent snapshot
var snapshot = hasher.copy()
```

## Test Vectors (FIPS 180-4)

| Input | Expected Digest |
|-------|----------------|
| `""` | `da39a3ee5e6b4b0d3255bfef95601890afd80709` |
| `"abc"` | `a9993e364706816aba3e25717850c26c9cd0d89d` |
| 56-byte message | `84983e441c3bd26ebaae4aa1f95129e5e54670f1` |

## Part Of

The [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.
