# BLAKE2b (Haskell)

Pure Haskell implementation of the **BLAKE2b** cryptographic hash
function (RFC 7693).  No external dependencies beyond `base`.

See the spec at [../../specs/HF06-blake2b.md](../../specs/HF06-blake2b.md)
for the full walk-through.

## Usage

```haskell
import Blake2b

-- One-shot, 64-byte digest
let h = blake2b (map (fromIntegral . fromEnum) "abc")
let s = blake2bHex (map (fromIntegral . fromEnum) "abc")

-- Parameterised (variable digest size, key, salt, personal)
let tag = blake2bHexWith
            defaultParams { digestSize = 32, key = sharedSecret }
            message
```

## API

| Function | Returns | Description |
|---|---|---|
| `blake2b :: [Word8] -> [Word8]` | bytes | One-shot 64-byte digest |
| `blake2bHex :: [Word8] -> String` | hex | One-shot lowercase hex digest |
| `blake2bWith :: Params -> [Word8] -> [Word8]` | bytes | With parameters |
| `blake2bHexWith :: Params -> [Word8] -> String` | hex | With parameters |
| `defaultParams :: Params` | `Params` | 64-byte digest, no key/salt/personal |

The `Params` record fields: `digestSize :: Int` (1..64), `key :: [Word8]`
(0..64 bytes), `salt :: [Word8]` (exactly 0 or 16 bytes), `personal ::
[Word8]` (exactly 0 or 16 bytes).

Invalid parameters raise a call to `error`, matching the exception-
raising behaviour of the sibling ports.  (A pure `Either` variant could
be layered on top without changing the core algorithm.)

## Implementation notes

Haskell's `Data.Word.Word64` and `Data.Bits.rotateR` map directly onto
the RFC's 64-bit ARX mixing.  The working vector is represented as a
plain `[Word64]` list of length 16; per-round updates use a small
`replaceMany` helper that rebuilds the list with four positions
overwritten.  For the KAT input sizes (up to 10 KiB) this is more than
fast enough and keeps the code structurally identical to the spec.

The 128-bit byte counter is represented as a single `Int` (effectively
64-bit on every platform where `ivsize == 8`).  The RFC's reserved
high 64 bits are always zero for any practical message.

## Scope

Sequential mode only.  Tree hashing, BLAKE2s, BLAKE2bp, BLAKE2sp,
BLAKE2Xb, and BLAKE3 are out of scope per the HF06 spec.  A streaming
`Hasher` value type is also deliberately omitted to keep the public
surface small — the one-shot API covers every KAT in the cross-
language suite.

## Running the tests

```bash
cabal test all
```

Tests cross-validate against fixed known-answer vectors precomputed from
Python's `hashlib.blake2b`.  The same KAT table is mirrored across every
language implementation in the monorepo.

## Part of coding-adventures

An educational computing stack built from logic gates up through
interpreters and compilers.  BLAKE2b is a prerequisite for Argon2
(the memory-hard password hashing function).
