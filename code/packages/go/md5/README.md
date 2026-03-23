# md5

MD5 message digest algorithm (RFC 1321) implemented from scratch in Go.

This package is part of the coding-adventures monorepo — a ground-up
exploration of the computing stack from transistors to operating systems.
The implementation does not use Go's `crypto/md5` standard library; every
step of the algorithm is written and explained here.

## Warning

MD5 is **cryptographically broken**. Practical collision attacks have existed
since 2004. Do NOT use this for passwords, digital signatures, or TLS
certificates. Valid uses: UUID v3 generation, non-security checksums, and
understanding how hash functions work at the bit level.

## How It Fits in the Stack

MD5 sits above the bit manipulation and arithmetic layers and below
cryptographic protocols. It demonstrates:

- How to process data in fixed-size blocks (512 bits)
- How padding extends a message to a block boundary
- How a compression function folds a block into a running state
- Why little-endian byte order matters and where bugs hide
- The Davies-Meyer feed-forward construction that prevents state inversion

## Algorithm Overview

MD5 maintains four 32-bit state words (A, B, C, D) initialized to constants
derived from counting in hex (01 23 45 67 / 89 AB CD EF / FE DC BA 98 /
76 54 32 10).

For each 64-byte (512-bit) block:

1. Parse the block as 16 **little-endian** 32-bit words `M[0..15]`.
2. Run 64 rounds split into four stages of 16. Each stage uses a different
   auxiliary function (F, G, H, I) and a different message-word access pattern.
3. Add the compressed output back to the input state (Davies-Meyer feed-forward).

After all blocks, serialize the four state words as **little-endian** bytes.
This little-endian serialization is the most common implementation pitfall.

### T-Table Constants

Each round adds a constant `T[i] = floor(abs(sin(i+1)) × 2³²)`. The use of
`sin` ensures the constants have no hidden mathematical structure ("nothing up
my sleeve"). The full derivation is in the source comments.

## Usage

### One-shot hash

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/md5"

// Returns a [16]byte fixed-size array.
digest := md5.SumMD5([]byte("hello, world"))

// Returns a 32-character lowercase hex string.
hex := md5.HexString([]byte("hello, world"))
fmt.Println(hex) // 3c6ef35b9e5343feabc06073b3f0d3ed — wait, compute it yourself :)
```

### Streaming hash

```go
d := md5.New()

// Write in as many chunks as you like. Order matters.
d.Write([]byte("hello, "))
d.Write([]byte("world"))

// Non-destructive — calling these methods does not reset the hasher.
digest := d.SumMD5()   // [16]byte
hex    := d.HexDigest() // string

// Continue writing after calling SumMD5/HexDigest.
d.Write([]byte("!"))
```

### As io.Writer

Because `Digest` implements `Write([]byte) (int, error)`, you can use it
anywhere an `io.Writer` is accepted:

```go
d := md5.New()
io.Copy(d, someReader)
fmt.Println(d.HexDigest())
```

## RFC 1321 Test Vectors

| Input | Digest |
|---|---|
| `""` | `d41d8cd98f00b204e9800998ecf8427e` |
| `"a"` | `0cc175b9c0f1b6a831c399e269772661` |
| `"abc"` | `900150983cd24fb0d6963f7d28e17f72` |
| `"message digest"` | `f96b697d7cb7938d525a2f31aaf161d0` |
| `"abcdefghijklmnopqrstuvwxyz"` | `c3fcd3d76192e4007dfb496cca67e13b` |

## Development

```bash
# Run tests with coverage
go test ./... -v -cover

# Lint
go vet ./...
```

Test coverage: **100%** of statements.
