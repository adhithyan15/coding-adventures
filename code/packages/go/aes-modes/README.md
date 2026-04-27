# aes-modes

AES modes of operation --- ECB, CBC, CTR, and GCM --- implemented from scratch in Go for educational purposes.

## What Are Modes of Operation?

AES is a **block cipher**: it encrypts exactly 16 bytes at a time. A **mode of operation** defines how to use the block cipher for messages of arbitrary length.

| Mode | Security | Description |
|------|----------|-------------|
| ECB  | BROKEN   | Each block encrypted independently. Patterns leak. |
| CBC  | Legacy   | Blocks chained via XOR. Vulnerable to padding oracles. |
| CTR  | Good     | Stream cipher mode. No padding. Parallelizable. |
| GCM  | Best     | Authenticated encryption. TLS 1.3 standard. |

## Usage

```go
import aesmodes "github.com/adhithyan15/coding-adventures/code/packages/go/aes-modes"

key := []byte{...} // 16, 24, or 32 bytes

// ECB (INSECURE)
ct, _ := aesmodes.EncryptECB(plaintext, key)
pt, _ := aesmodes.DecryptECB(ct, key)

// CBC
iv := make([]byte, 16) // Use crypto/rand in production
ct, _ = aesmodes.EncryptCBC(plaintext, key, iv)
pt, _ = aesmodes.DecryptCBC(ct, key, iv)

// CTR
nonce := make([]byte, 12) // Use crypto/rand in production
ct, _ = aesmodes.EncryptCTR(plaintext, key, nonce)
pt, _ = aesmodes.DecryptCTR(ct, key, nonce)

// GCM (authenticated encryption)
ct, tag, _ := aesmodes.EncryptGCM(plaintext, key, iv12, aad)
pt, _ = aesmodes.DecryptGCM(ct, key, iv12, aad, tag)
```

## Dependencies

- `go/aes` --- AES block cipher (EncryptBlock, DecryptBlock)
- `go/gf256` --- GF(2^8) arithmetic (transitive dependency via aes)

## Testing

```bash
go test ./... -v -cover
```

Tests use NIST SP 800-38A vectors (ECB, CBC, CTR) and NIST GCM specification vectors.
