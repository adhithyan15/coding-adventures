# argon2i (Haskell)

Pure-Haskell implementation of **Argon2i** (RFC 9106) — the
data-independent variant of the Argon2 memory-hard password-hashing
and key-derivation function.

Argon2i picks reference blocks from a deterministic address stream that
only depends on public parameters (pass, lane, slice, m', total time,
type, counter) — never on the password. That eliminates the timing side
channel that Argon2d has, at the cost of some GPU/ASIC resistance.

For general password hashing, prefer the hybrid
[`argon2id`](../argon2id). Use this package when side-channel safety is
explicitly required and you are comfortable with the performance
trade-off.

## Usage

```haskell
import Argon2i (argon2i, argon2iHex, argon2Version)

main :: IO ()
main = do
    let password = replicate 32 0x01
        salt     = replicate 16 0x02
        key      = replicate 8  0x03
        ad       = replicate 12 0x04
    putStrLn $ argon2iHex password salt 3 32 4 32 key ad argon2Version
    -- c814d9d1dc7f37aa13f0d77f2494bda1c8de6b016dd388d29952a4c4672b6ce8
```

Parameters, validation rules, and trust boundary match the sibling
`argon2d` package.

## Dependencies

- `base`
- `array`
- sibling [`blake2b`](../blake2b)
