# sha1 (Go)

SHA-1 cryptographic hash function (FIPS 180-4) implemented from scratch in Go.

## What It Does

SHA-1 takes any sequence of bytes and produces a fixed-size 20-byte (160-bit) digest.
The same input always yields the same digest. Change one bit of input and the entire
digest changes — the avalanche effect. This package implements SHA-1 from scratch,
without using Go's `crypto/sha1`, so every step of the algorithm is visible.

## How It Fits in the Stack

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
monorepo. SHA-1 is a prerequisite for the UUID v5 package.

SHA-1 is no longer considered collision-resistant (the SHAttered attack, 2017) and should
not be used for security-critical purposes. It remains valid for UUID v5 and Git object IDs.

## API

`Sum1` is named with a `1` suffix to avoid clashing with the stdlib's `crypto/sha1.Sum`.

```go
import "sha1"

// One-shot
digest := sha1.Sum1([]byte("abc"))        // [20]byte
hex := sha1.HexString([]byte("abc"))      // "a9993e364706816aba3e25717850c26c9cd0d89d"

// Streaming
h := sha1.New()
h.Write([]byte("ab"))
h.Write([]byte("c"))
fmt.Println(h.HexDigest())               // "a9993e364706816aba3e25717850c26c9cd0d89d"
```

## FIPS 180-4 Test Vectors

```go
assert sha1.HexString([]byte("")) == "da39a3ee5e6b4b0d3255bfef95601890afd80709"
assert sha1.HexString([]byte("abc")) == "a9993e364706816aba3e25717850c26c9cd0d89d"
```

## Development

```bash
go test ./... -v -cover
```
