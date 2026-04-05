# CodingAdventures.ReedSolomon

Reed-Solomon error-correcting codes over GF(256). Elixir implementation of [MA02](../../../specs/MA02-reed-solomon.md).

The math behind QR codes, CDs, DVDs, deep-space communication, and RAID-6.

## Quick Start

```elixir
alias CodingAdventures.ReedSolomon, as: RS

message = String.to_charlist("Hello, World!")
n_check = 8  # t = 4 errors correctable

# Encode: systematic — message bytes unchanged, check bytes appended
codeword = RS.encode(message, n_check)
# length(codeword) == length(message) + 8

# Corrupt 4 bytes — still recoverable
import Bitwise
codeword =
  codeword
  |> List.replace_at(0, bxor(Enum.at(codeword, 0), 0xFF))
  |> List.replace_at(3, bxor(Enum.at(codeword, 3), 0xAA))
  |> List.replace_at(7, bxor(Enum.at(codeword, 7), 0x55))
  |> List.replace_at(10, bxor(Enum.at(codeword, 10), 0x0F))

recovered = RS.decode(codeword, n_check)
# recovered == message
```

## API

### `RS.encode(message, n_check) → list(integer)`

Encode `message` (list of byte integers) with `n_check` redundancy bytes. Returns a
list of length `length(message) + n_check`. The first `length(message)` bytes are
the original message (systematic encoding).

**Raises** `ReedSolomon.InvalidInput` if `n_check` is 0, odd, or total length > 255.

### `RS.decode(received, n_check) → list(integer)`

Decode a (possibly corrupted) codeword. Returns the recovered message bytes.
Corrects up to `t = n_check / 2` byte errors.

**Raises** `ReedSolomon.TooManyErrors` if more than `t` errors are present.
**Raises** `ReedSolomon.InvalidInput` if `n_check` is 0/odd or received is too short.

### `RS.syndromes(received, n_check) → list(integer)`

Compute the `n_check` syndrome values `S_j = received(α^j)` for `j = 1…n_check`.
All-zero → no errors; any non-zero → errors detected.

### `RS.build_generator(n_check) → list(integer)`

Build the monic generator polynomial `g(x) = ∏(x + αⁱ)` for `i = 1…n_check`.
Returns a little-endian list of length `n_check + 1`.

### `RS.error_locator(syndromes) → list(integer)`

Compute the error locator polynomial `Λ(x)` from a syndrome list using
Berlekamp-Massey. Returns `Λ` in little-endian form with `Λ[0] = 1`.

## Error Types

```elixir
CodingAdventures.ReedSolomon.TooManyErrors  # > t errors; unrecoverable
CodingAdventures.ReedSolomon.InvalidInput   # bad n_check or codeword too long
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
MA00  gf256        (elixir/gf256)
MA01  polynomial   (elixir/polynomial)
MA02  reed_solomon ← this package
```

## Specification

See [`code/specs/MA02-reed-solomon.md`](../../../specs/MA02-reed-solomon.md).
