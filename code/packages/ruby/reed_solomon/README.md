# CodingAdventures::ReedSolomon

Reed-Solomon error-correcting codes over GF(256). Ruby implementation of [MA02](../../../specs/MA02-reed-solomon.md).

The math behind QR codes, CDs, DVDs, deep-space communication, and RAID-6.

## Quick Start

```ruby
require_relative "lib/reed_solomon"

message = "Hello, World!".bytes
n_check = 8  # t = 4 errors correctable

# Encode: systematic — message bytes unchanged, check bytes appended
codeword = ReedSolomon.encode(message, n_check)
# codeword.length == message.length + 8

# Corrupt 4 bytes — still recoverable
codeword[0] ^= 0xFF
codeword[3] ^= 0xAA
codeword[7] ^= 0x55
codeword[10] ^= 0x0F

recovered = ReedSolomon.decode(codeword, n_check)
# recovered == message
```

## API

### `ReedSolomon.encode(message, n_check) → Array<Integer>`

Encode `message` (array of bytes) with `n_check` redundancy bytes. Returns an
array of length `message.length + n_check`. The first `message.length` bytes are
the original message (systematic encoding).

**Raises** `ReedSolomon::InvalidInput` if `n_check` is 0, odd, or total length > 255.

### `ReedSolomon.decode(received, n_check) → Array<Integer>`

Decode a (possibly corrupted) codeword. Returns the recovered message bytes.
Corrects up to `t = n_check / 2` byte errors.

**Raises** `ReedSolomon::TooManyErrors` if more than `t` errors are present.
**Raises** `ReedSolomon::InvalidInput` if `n_check` is 0/odd or received is too short.

### `ReedSolomon.syndromes(received, n_check) → Array<Integer>`

Compute the `n_check` syndrome values `S_j = received(α^j)` for `j = 1…n_check`.
All-zero → no errors; any non-zero → errors detected.

### `ReedSolomon.build_generator(n_check) → Array<Integer>`

Build the monic generator polynomial `g(x) = ∏(x + αⁱ)` for `i = 1…n_check`.
Returns a little-endian `Array<Integer>` of length `n_check + 1`.

### `ReedSolomon.error_locator(syndromes) → Array<Integer>`

Compute the error locator polynomial `Λ(x)` from a syndrome array using
Berlekamp-Massey. Returns `Λ` in little-endian form with `Λ[0] = 1`.

## Error Classes

```ruby
ReedSolomon::TooManyErrors < StandardError  # > t errors; unrecoverable
ReedSolomon::InvalidInput  < ArgumentError  # bad n_check or codeword too long
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
MA00  gf256        (ruby/gf256)
MA01  polynomial   (ruby/polynomial)
MA02  reed_solomon ← this package
```

## Specification

See [`code/specs/MA02-reed-solomon.md`](../../../specs/MA02-reed-solomon.md).
