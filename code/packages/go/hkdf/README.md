# hkdf

HKDF (HMAC-based Extract-and-Expand Key Derivation Function) implemented from scratch following RFC 5869.

## What Is HKDF?

HKDF derives one or more cryptographically strong keys from input keying material (IKM). It is the standard key derivation function used in TLS 1.3, Signal Protocol, WireGuard, and many other modern protocols.

## Usage

```go
import hkdf "github.com/adhithyan15/coding-adventures/code/packages/go/hkdf"

// Combined extract-then-expand (most common usage)
okm, err := hkdf.HKDF(salt, ikm, info, 32, hkdf.SHA256)

// Separate extract and expand
prk := hkdf.Extract(salt, ikm, hkdf.SHA256)
okm, err := hkdf.Expand(prk, info, 32, hkdf.SHA256)

// SHA-512 variant
okm, err := hkdf.HKDF(salt, ikm, info, 64, hkdf.SHA512)
```

## API

- `Extract(salt, ikm, algorithm) []byte` -- Extract phase
- `Expand(prk, info, length, algorithm) ([]byte, error)` -- Expand phase
- `HKDF(salt, ikm, info, length, algorithm) ([]byte, error)` -- Combined

## Dependencies

- `hmac` (which depends on `sha256`, `sha512`, etc.)

## How It Fits in the Stack

```
HKDF (this package)
  +-- HMAC
        +-- SHA-256
        +-- SHA-512
```
