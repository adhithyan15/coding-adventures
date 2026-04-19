# blake2b

A pure-Go, from-scratch implementation of the **BLAKE2b** cryptographic hash
function (RFC 7693). Zero external dependencies.

## What is BLAKE2b?

BLAKE2b is a modern hash function that is:

- Faster than MD5 on 64-bit hardware.
- As secure as SHA-3 against known attacks.
- Variable output length (1..64 bytes).
- Keyed in a single pass (replaces HMAC-SHA-512).
- Parameterized with salt and personalization.

It is the hash underlying libsodium, WireGuard, Noise Protocol, IPFS
content addressing, and -- within this repo -- Argon2.

See the spec at [code/specs/HF06-blake2b.md](../../../specs/HF06-blake2b.md)
for the full algorithm walkthrough.

## Usage

### One-shot

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/blake2b"

digest, err := blake2b.Sum([]byte("abc"), 64, nil, nil, nil)
// 64-byte digest

hexStr, _ := blake2b.SumHex([]byte("abc"), 32, nil, nil, nil)
// 64-char hex string (32-byte digest)
```

### Keyed (MAC mode)

```go
mac, _ := blake2b.Sum(message, 32, []byte("shared secret"), nil, nil)
```

### Streaming

```go
h, _ := blake2b.New(32, nil, nil, nil)
h.Update([]byte("partial "))
h.Update([]byte("payload"))
fmt.Println(h.HexDigest())
```

### Salt and personalization

Two applications that share a secret can domain-separate their MACs by
setting distinct 16-byte `personal` strings.

```go
h, _ := blake2b.New(32, key, salt, []byte("app-A-mac-v1....")) // 16 bytes
```

## Where this fits in the stack

- **Dependencies:** none.
- **Used by:** Argon2 (forthcoming).

## Scope

Sequential mode only. Tree hashing, BLAKE2s, BLAKE2bp, BLAKE2sp,
BLAKE2Xb, and BLAKE3 are intentionally not included -- see the
"Non-Goals" section of the spec.

## Running the tests

```bash
go test ./... -v -cover
```

Tests cross-validate every path against fixed known-answer vectors
computed from Python's `hashlib.blake2b`, which wraps the reference
implementation.
