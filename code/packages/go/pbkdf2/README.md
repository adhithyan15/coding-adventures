# go/pbkdf2

PBKDF2 (Password-Based Key Derivation Function 2) — RFC 8018 — implemented from scratch in Go.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/pbkdf2"

dk, err := pbkdf2.PBKDF2HmacSHA256(
    []byte("correct horse battery staple"),
    []byte{0xde, 0xad, 0xbe, 0xef, /* ... 16 random bytes */},
    600_000,  // OWASP 2023 minimum for SHA-256
    32,
)
```

## API

| Function                  | PRF         | Returns          |
|---------------------------|-------------|------------------|
| `PBKDF2HmacSHA1`          | HMAC-SHA1   | `([]byte, error)`|
| `PBKDF2HmacSHA256`        | HMAC-SHA256 | `([]byte, error)`|
| `PBKDF2HmacSHA512`        | HMAC-SHA512 | `([]byte, error)`|
| `PBKDF2HmacSHA1Hex`       | HMAC-SHA1   | `(string, error)`|
| `PBKDF2HmacSHA256Hex`     | HMAC-SHA256 | `(string, error)`|
| `PBKDF2HmacSHA512Hex`     | HMAC-SHA512 | `(string, error)`|

## Errors

- `ErrEmptyPassword` — password has zero length
- `ErrInvalidIterations` — iterations ≤ 0
- `ErrInvalidKeyLength` — key_length ≤ 0

## Stack Position

KD01. Depends on `go/hmac` (HF05), which depends on `go/md5`, `go/sha1`, `go/sha256`, `go/sha512`.
