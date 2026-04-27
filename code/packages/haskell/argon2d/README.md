# argon2d (Haskell)

Pure-Haskell implementation of **Argon2d** (RFC 9106) — the
data-dependent variant of the Argon2 memory-hard password-hashing and
key-derivation function.

Argon2 won the 2015 Password Hashing Competition. Argon2d is the
"maximum ASIC resistance" variant: the index of every reference block
is derived from the previous block's contents, which makes memory-access
patterns correlated with the password. That wins cost per guess on
GPUs/FPGAs but leaks a timing side channel — so use Argon2d only when
side-channel attacks are outside the threat model. Prime example:
proof-of-work style schedules where the inputs are public. For general
password hashing prefer sibling `argon2id`.

## Usage

```haskell
import Argon2d (argon2d, argon2dHex, argon2Version)

main :: IO ()
main = do
    let password = replicate 32 0x01
        salt     = replicate 16 0x02
        key      = replicate 8  0x03
        ad       = replicate 12 0x04
    putStrLn $ argon2dHex password salt 3 32 4 32 key ad argon2Version
    -- 512b391b6f1162975371d30919734294f868e3be3984f3c1a13a4db9fabe4acb
```

The API returns `[Word8]` (or hex via `argon2dHex`); use sibling
`blake2b`'s helpers or your own encoder to convert to/from strings.

### Parameters

| Argument         | Meaning                                            |
| ---------------- | -------------------------------------------------- |
| `password`       | `[Word8]`; any length ≤ 2³²−1 bytes                |
| `salt`           | `[Word8]`; **≥ 8 bytes** (16+ recommended)         |
| `timeCost`       | Number of passes over memory (≥ 1)                 |
| `memoryCost`     | KiB of memory (≥ `8 * parallelism`)                |
| `parallelism`    | Lanes (`[1, 2²⁴−1]`)                               |
| `tagLength`      | Output length in bytes (≥ 4)                       |
| `key`            | Optional MAC secret (`[]` if none)                 |
| `associatedData` | Optional context bytes (`[]` if none)              |
| `version`        | Must be `argon2Version` (`0x13`)                   |

## Trust boundary

All length inputs must fit in 32 bits; this is validated at the entry
points. DoS from oversized `memory_cost` or `time_cost` is the caller's
responsibility — the library performs exactly the work requested.

## Dependencies

- `base`
- `array` (for `UArray Word64` blocks and the boxed memory matrix)
- sibling [`blake2b`](../blake2b) package

## Fit in the stack

This package is a leaf primitive under `code/packages/haskell/`. It
powers the Haskell strand of higher-level constructions in Vault and
any language-parity test harnesses. Siblings:

- [`argon2i`](../argon2i) — data-independent variant
- [`argon2id`](../argon2id) — hybrid, recommended default
