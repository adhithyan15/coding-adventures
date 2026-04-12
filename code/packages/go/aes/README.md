# aes — AES Block Cipher (Go)

A pure-Go educational implementation of the Advanced Encryption Standard (AES),
conforming to FIPS 197. Supports all three key sizes: AES-128, AES-192, AES-256.

This package is for education. Production code should use `crypto/aes` from the
Go standard library, which leverages AES-NI hardware acceleration.

## What It Implements

- `ExpandKey` — expand a 16/24/32-byte key into Nr+1 round keys
- `EncryptBlock` — encrypt one 16-byte block
- `DecryptBlock` — decrypt one 16-byte block
- `SBOX`, `INV_SBOX` — the AES substitution tables, built at init time

## How It Fits in the Stack

Layer SE01 (symmetric encryption primitives). Depends on `go/gf256` for
GF(2^8) arithmetic. Companion to `go/des`.

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/aes"

key := make([]byte, 16)  // AES-128
plain := make([]byte, 16)

ct, err := aes.EncryptBlock(plain, key)
pt, err := aes.DecryptBlock(ct, key)

// Access the S-box directly
fmt.Printf("SBOX[0x00] = %02x\n", aes.SBOX[0])  // 0x63
```

## Algorithm Notes

AES is a Substitution-Permutation Network (SPN) operating on a 4×4 byte state
matrix (loaded column-major from the 16-byte block).

Each full round applies four transformations:

1. **SubBytes** — replace each byte with `SBOX[b]` (GF(2^8) inverse + affine transform)
2. **ShiftRows** — cyclically shift row `i` left by `i` positions
3. **MixColumns** — multiply each column by the AES matrix in GF(2^8)
4. **AddRoundKey** — XOR with the current round key

The final round omits MixColumns. Decryption applies the inverse of each step
in reverse order with reversed round keys.

### GF(2^8) Field

AES uses polynomial `0x11B = x^8 + x^4 + x^3 + x + 1`. The S-box maps each
byte to its multiplicative inverse in this field, then applies an affine
transformation over GF(2). This two-step design provides non-linearity and
eliminates fixed points.

## Key Sizes

| Key size  | Nk | Nr | Round keys |
|-----------|----|----|------------|
| 128 bits  | 4  | 10 | 11         |
| 192 bits  | 6  | 12 | 13         |
| 256 bits  | 8  | 14 | 15         |

## Test Vectors

| Version  | Key (hex)                          | Plaintext (hex)                  | Ciphertext (hex)                 |
|----------|------------------------------------|----------------------------------|----------------------------------|
| AES-128  | `2b7e...4f3c`                      | `3243...0734`                    | `3925...0b32`                    |
| AES-192  | `0001...1617`                      | `0011...eeff`                    | `dda9...7191`                    |
| AES-256  | `603d...dff4`                      | `6bc1...172a`                    | `f3ee...81f8`                    |

All from FIPS 197 Appendix B/C.

## Running Tests

```bash
go test ./... -v -cover
```
