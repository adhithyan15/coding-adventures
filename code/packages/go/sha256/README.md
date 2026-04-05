# sha256

SHA-256 cryptographic hash function (FIPS 180-4) implemented from scratch in Go.

## What Is SHA-256?

SHA-256 is a cryptographic hash function from the SHA-2 family that produces a 256-bit (32-byte) digest. It is the workhorse of modern cryptography, used in TLS, Bitcoin, git, code signing, and password hashing.

This package implements SHA-256 from first principles with no dependencies on `crypto/sha256` or any other cryptographic library.

## How It Fits in the Stack

This is package HF03 in the coding-adventures monorepo (Go variant). It builds on the same Merkle-Damgard construction used in the SHA-1 package, but with a wider state (8 words), more complex message schedule, and stronger auxiliary functions.

## Usage

### One-shot hashing

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/sha256"

digest := sha256.Sum256([]byte("hello world"))    // [32]byte
hexStr := sha256.HexString([]byte("hello world")) // 64-char hex string
```

### Streaming (chunked) hashing

```go
h := sha256.New()
h.Write([]byte("hello "))
h.Write([]byte("world"))
fmt.Println(h.HexDigest()) // same as HexString([]byte("hello world"))
```

### Branching with Copy()

```go
h := sha256.New()
h.Write([]byte("common prefix"))
h1 := h.Copy()
h2 := h.Copy()
h1.Write([]byte(" branch A"))
h2.Write([]byte(" branch B"))
// h1 and h2 have different digests
```

## API

| Function / Type | Description |
|---|---|
| `Sum256(data) [32]byte` | One-shot hash |
| `HexString(data) string` | One-shot hex result |
| `New() *Digest` | Create streaming hasher |
| `(*Digest).Write(p)` | Feed bytes |
| `(*Digest).Sum256() [32]byte` | Get digest (non-destructive) |
| `(*Digest).HexDigest() string` | Get hex digest |
| `(*Digest).Copy() *Digest` | Deep clone |

## Testing

```bash
go test ./... -v -cover
```
