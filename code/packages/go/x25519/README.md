# X25519 — Elliptic Curve Diffie-Hellman (RFC 7748)

A zero-dependency Go implementation of the X25519 key agreement protocol. All field arithmetic over GF(2^255 - 19) uses Go's `math/big` package.

## Usage

```go
import "github.com/example/coding-adventures/code/packages/go/x25519"

// Generate keypairs
alicePublic, _ := x25519.GenerateKeypair(alicePrivate)
bobPublic, _ := x25519.GenerateKeypair(bobPrivate)

// Derive shared secret
sharedAB, _ := x25519.X25519(alicePrivate, bobPublic)
sharedBA, _ := x25519.X25519(bobPrivate, alicePublic)
// sharedAB == sharedBA
```

## API

- `X25519(scalar, uCoord [32]byte) ([32]byte, error)` — Core scalar multiplication
- `X25519Base(scalar [32]byte) ([32]byte, error)` — Multiply by base point (u=9)
- `GenerateKeypair(privateKey [32]byte) ([32]byte, error)` — Derive public key

## Testing

```bash
go test ./... -v -cover
```

All RFC 7748 test vectors pass, including the iterated 1000-round test.
