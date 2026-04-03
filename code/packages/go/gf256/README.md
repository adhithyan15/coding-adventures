# go/gf256

Galois Field GF(2^8) arithmetic. 256-element field for Reed-Solomon,
QR codes, and AES encryption.

## Stack Position

Layer MA01 — enables MA02 (reed-solomon) and MA03 (qr-encoder).

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/gf256"

gf256.Add(0x53, 0xCA)        // → 0x99 (XOR)
gf256.Multiply(0x53, 0xCA)   // → 0x01 (inverses!)
gf256.Inverse(0x53)          // → 0xCA
gf256.Power(2, 255)          // → 1 (g^255 = 1)
```

## API

- `Add(a, b)`, `Subtract(a, b)` — XOR
- `Multiply(a, b)`, `Divide(a, b)` — via log/antilog tables
- `Power(base, exp)` — exponentiation
- `Inverse(a)` — multiplicative inverse; panics for a=0
- `Zero()`, `One()` — field identities
- `LOG()`, `ALOG()` — table accessors
