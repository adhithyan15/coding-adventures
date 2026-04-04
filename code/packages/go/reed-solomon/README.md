# reedsolomon

Reed-Solomon error-correcting codes over GF(256). Go implementation of [MA02](../../../specs/MA02-reed-solomon.md).

The math behind QR codes, CDs, DVDs, deep-space communication, and RAID-6.

## Install

```bash
go get github.com/adhithyan15/coding-adventures/code/packages/go/reed-solomon
```

## Quick Start

```go
import rs "github.com/adhithyan15/coding-adventures/code/packages/go/reed-solomon"

message := []byte("Hello, World!")
nCheck := 8  // t = 4 errors correctable

// Encode: systematic — message bytes unchanged, check bytes appended
codeword, _ := rs.Encode(message, nCheck)
// len(codeword) == len(message) + 8

// Corrupt 4 bytes — still recoverable
codeword[0] ^= 0xFF
codeword[3] ^= 0xAA
codeword[7] ^= 0x55
codeword[10] ^= 0x0F

recovered, err := rs.Decode(codeword, nCheck)
// recovered deep-equals message
```

## API

### `Encode(message []byte, nCheck int) ([]byte, error)`

Encode `message` with `nCheck` redundancy bytes. Returns a `[]byte` of length
`len(message) + nCheck`. The first `len(message)` bytes are the original message
(systematic encoding).

Returns `*InvalidInputError` if `nCheck` is 0, odd, or `len(message)+nCheck > 255`.

### `Decode(received []byte, nCheck int) ([]byte, error)`

Decode a (possibly corrupted) codeword. Returns the recovered message bytes.
Corrects up to `t = nCheck/2` byte errors.

Returns `ErrTooManyErrors` if more than `t` errors are present.
Returns `*InvalidInputError` if `nCheck` is 0/odd or received is too short.

### `Syndromes(received []byte, nCheck int) []byte`

Compute the `nCheck` syndrome values `S_j = received(α^j)` for `j = 1…nCheck`.
All-zero → no errors; any non-zero → errors detected.

### `BuildGenerator(nCheck int) ([]byte, error)`

Build the monic generator polynomial `g(x) = ∏(x + αⁱ)` for `i = 1…nCheck`.
Returns a little-endian `[]byte` of length `nCheck + 1`.

### `ErrorLocator(synds []byte) []byte`

Compute the error locator polynomial `Λ(x)` from a syndrome slice using
Berlekamp-Massey. Returns `Λ` in little-endian form with `Λ[0] = 1`.

## Error Types

```go
var ErrTooManyErrors = errors.New(...)  // > t errors; unrecoverable

type InvalidInputError struct{ Reason string }
```

## Correction Capacity

| `nCheck` | `t` (errors correctable) |
|----------|--------------------------|
| 2        | 1                        |
| 4        | 2                        |
| 8        | 4                        |
| 16       | 8                        |
| 32       | 16                       |

## Stack

```
MA00  github.com/adhithyan15/coding-adventures/code/packages/go/polynomial
MA01  github.com/adhithyan15/coding-adventures/code/packages/go/gf256
MA02  github.com/adhithyan15/coding-adventures/code/packages/go/reed-solomon  ← this package
```

## Specification

See [`code/specs/MA02-reed-solomon.md`](../../../specs/MA02-reed-solomon.md).
