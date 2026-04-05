# MD5 (Swift)

Pure Swift implementation of the MD5 message digest algorithm (RFC 1321) with no external dependencies.

## What It Does

MD5 takes any sequence of bytes and produces a fixed-size 16-byte (128-bit) digest. The same input always produces the same digest. This implementation provides both one-shot and streaming APIs.

**WARNING:** MD5 is cryptographically broken (collision attacks since 2004). Do NOT use for security. Valid uses: checksums, UUID v3, legacy compatibility.

## Where It Fits

```
Application / UUID v3
      |
      v
  md5(data) --> 16-byte digest
      |
      v
  [This package -- no dependencies]
```

## API

### One-Shot

```swift
import MD5

// Returns 16-byte Data
let digest = md5(Data("abc".utf8))

// Returns 32-character hex string
let hex = md5Hex(Data("abc".utf8))
// "900150983cd24fb0d6963f7d28e17f72"
```

### Streaming

```swift
var hasher = MD5Hasher()
hasher.update(Data("ab".utf8))
hasher.update(Data("c".utf8))
hasher.hexDigest()  // "900150983cd24fb0d6963f7d28e17f72"

// Non-destructive: digest() can be called multiple times
// copy() creates an independent snapshot
var snapshot = hasher.copy()
```

## Test Vectors (RFC 1321)

| Input | Expected Digest |
|-------|----------------|
| `""` | `d41d8cd98f00b204e9800998ecf8427e` |
| `"abc"` | `900150983cd24fb0d6963f7d28e17f72` |
| `"message digest"` | `f96b697d7cb7938d525a2f31aaf161d0` |

## Part Of

The [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.
