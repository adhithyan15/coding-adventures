# ChaCha20-Poly1305 (Go)

A from-scratch implementation of the ChaCha20-Poly1305 AEAD cipher suite
(RFC 8439), using only ARX (Add, Rotate, XOR) operations.

## What's Inside

- **ChaCha20** stream cipher: 256-bit key, 96-bit nonce, 32-bit counter
- **Poly1305** one-time MAC: 16-byte authentication tag (uses math/big for 130-bit arithmetic)
- **AEAD** construction: combined authenticated encryption per RFC 8439

## Usage

```go
import chacha20poly1305 "github.com/adhithyan15/coding-adventures/code/packages/go/chacha20-poly1305"

// Stream cipher
ct, err := chacha20poly1305.ChaCha20Encrypt(plaintext, key, nonce, 0)

// One-time MAC
tag, err := chacha20poly1305.Poly1305Mac(message, key)

// Authenticated encryption
ct, tag, err := chacha20poly1305.AEADEncrypt(plaintext, key, nonce, aad)
pt, err := chacha20poly1305.AEADDecrypt(ct, key, nonce, aad, tag)
```

## Building

```bash
go test ./... -v -cover
```

## Part Of

[coding-adventures](https://github.com/adhithyan15/coding-adventures) -- a
monorepo of from-scratch implementations for learning.
