# Ed25519 Digital Signatures (Go)

A from-scratch implementation of Ed25519 digital signatures (RFC 8032) in Go,
using `math/big` for field and scalar arithmetic.

## What Is Ed25519?

Ed25519 is an elliptic curve digital signature algorithm (EdDSA) operating on the
twisted Edwards curve -x^2 + y^2 = 1 + d*x^2*y^2 over GF(2^255 - 19). It provides
32-byte keys, 64-byte signatures, 128-bit security, and deterministic signing.

## Usage

```go
import ed25519 "github.com/example/coding-adventures/code/packages/go/ed25519"

seed := [32]byte{ /* 32 random bytes */ }
publicKey, secretKey := ed25519.GenerateKeypair(seed)
signature := ed25519.Sign([]byte("hello"), secretKey)
ok := ed25519.Verify([]byte("hello"), signature, publicKey)
```

## Dependencies

- `github.com/example/coding-adventures/code/packages/go/sha512` — our from-scratch SHA-512

## How It Fits

This package builds on SHA-512 and provides signing alongside X25519 key exchange
in the Curve25519 family.

## Testing

```sh
go test ./... -v -cover
```

All four RFC 8032 Section 7.1 test vectors plus field arithmetic, point operation,
encoding/decoding, and rejection tests.
