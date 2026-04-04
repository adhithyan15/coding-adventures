# coding-adventures-reed-solomon

Reed-Solomon error-correcting codes over GF(256). Python implementation of [MA02](../../../specs/MA02-reed-solomon.md).

The math behind QR codes, CDs, DVDs, deep-space communication, and RAID-6.

## Install

```bash
pip install coding-adventures-reed-solomon
```

## Quick Start

```python
from reed_solomon import encode, decode

message = b"Hello, World!"
n_check = 8  # t = 4 errors correctable

# Encode: systematic — message bytes unchanged, check bytes appended
codeword = encode(message, n_check)
print(len(codeword))  # len(message) + 8

# Corrupt 4 bytes — still recoverable
cw = bytearray(codeword)
cw[0] ^= 0xFF
cw[3] ^= 0xAA
cw[7] ^= 0x55
cw[10] ^= 0x0F

recovered = decode(bytes(cw), n_check)
assert recovered == message
```

## API

### `encode(message, n_check) → bytes`

Encode `message` with `n_check` redundancy bytes. Returns `bytes` of length
`len(message) + n_check`. The first `len(message)` bytes are the original
message (systematic encoding).

**Raises** `InvalidInputError` if `n_check` is 0, odd, or `len(message) + n_check > 255`.

### `decode(received, n_check) → bytes`

Decode a (possibly corrupted) codeword. Returns the recovered message bytes.
Corrects up to `t = n_check // 2` byte errors.

**Raises** `TooManyErrorsError` if more than `t` errors are present.
**Raises** `InvalidInputError` if `n_check` is 0/odd or received is too short.

### `syndromes(received, n_check) → list[int]`

Compute the `n_check` syndrome values `S_j = received(α^j)` for `j = 1…n_check`.
All-zero → no errors; any non-zero → errors detected.

### `build_generator(n_check) → list[int]`

Build the monic generator polynomial `g(x) = ∏(x + αⁱ)` for `i = 1…n_check`.
Returns a little-endian `list[int]` of length `n_check + 1`.

### `error_locator(syndromes) → list[int]`

Compute the error locator polynomial `Λ(x)` from a syndrome list using
Berlekamp-Massey. Returns `Λ` in little-endian form with `Λ[0] = 1`.

## Error Classes

```python
class TooManyErrorsError(Exception): ...  # > t errors; unrecoverable
class InvalidInputError(Exception): ...   # bad n_check or codeword too long
```

## Correction Capacity

| `n_check` | `t` (errors correctable) |
|-----------|--------------------------|
| 2         | 1                        |
| 4         | 2                        |
| 8         | 4                        |
| 16        | 8                        |
| 32        | 16                       |

## Stack

```
MA00  coding-adventures-polynomial
MA01  coding-adventures-gf256
MA02  coding-adventures-reed-solomon  ← this package
```

## Specification

See [`code/specs/MA02-reed-solomon.md`](../../../specs/MA02-reed-solomon.md).
