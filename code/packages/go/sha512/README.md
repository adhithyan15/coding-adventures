# go/sha512

SHA-512 cryptographic hash function (FIPS 180-4) implemented from scratch in Go.

## What Is SHA-512?

SHA-512 is the 64-bit sibling of SHA-256 in the SHA-2 family. It produces a 512-bit (64-byte) digest using 8 x 64-bit state words and 80 rounds. On 64-bit platforms it is often faster than SHA-256 because it processes 128-byte blocks.

## How It Fits

Part of the `coding-adventures` monorepo hash function collection, alongside MD5, SHA-1, and SHA-256. Implemented from scratch with literate programming style for learning.

## Usage

```go
import sha512 "github.com/adhithyan15/coding-adventures/code/packages/go/sha512"

// One-shot
digest := sha512.Sum512([]byte("abc"))    // [64]byte
hexStr := sha512.HexString([]byte("abc")) // 128-char string

// Streaming
h := sha512.New()
h.Write([]byte("ab"))
h.Write([]byte("c"))
fmt.Println(h.HexDigest())
```

## API

| Function | Returns | Description |
|----------|---------|-------------|
| `Sum512(data)` | `[64]byte` | 64-byte digest |
| `HexString(data)` | `string` | 128-char lowercase hex |
| `New()` | `*Digest` | Streaming hasher |
| `(*Digest).Write(p)` | `(int, error)` | Feed bytes |
| `(*Digest).Sum512()` | `[64]byte` | Get digest (non-destructive) |
| `(*Digest).HexDigest()` | `string` | Get hex (non-destructive) |

## Development

```bash
go test ./... -v -cover
```
