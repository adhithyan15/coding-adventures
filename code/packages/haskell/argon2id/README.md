# argon2id (Haskell)

Pure-Haskell implementation of **Argon2id** (RFC 9106) — the hybrid
variant of the Argon2 memory-hard password-hashing and key-derivation
function, recommended by the RFC as the default for most password
hashing use cases.

Argon2id uses Argon2i's data-INDEPENDENT indexing for the first two
slices of the first pass (where side-channel exposure is largest,
because the attacker sees those derivations earliest) and Argon2d's
data-DEPENDENT indexing for the remaining segments (which maximises
GPU/ASIC resistance once the side-channel window has closed).

## Usage

```haskell
import Argon2id (argon2id, argon2idHex, argon2Version)

main :: IO ()
main = do
    let password = replicate 32 0x01
        salt     = replicate 16 0x02
        key      = replicate 8  0x03
        ad       = replicate 12 0x04
    putStrLn $ argon2idHex password salt 3 32 4 32 key ad argon2Version
    -- 0d640df58d78766c08c037a34a8b53c9d01ef0452d75b65eb52520e96b01e659
```

Parameters, validation rules, and trust boundary match the sibling
`argon2d` / `argon2i` packages.

## Dependencies

- `base`
- `array`
- sibling [`blake2b`](../blake2b)
