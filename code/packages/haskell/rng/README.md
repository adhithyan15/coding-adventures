# coding-adventures-rng (Haskell)

Three classic pseudorandom number generators implemented in idiomatic Haskell:
**LCG** (Knuth 1948), **Xorshift64** (Marsaglia 2003), and **PCG32** (O'Neill 2014).

All three implement the `RandomGen` typeclass and share a uniform API.
Because Haskell is pure, each generator operation returns the new generator
state alongside the output value rather than mutating state in place.

## Algorithms

| Generator  | Year | Core idea                                     | Period     |
|------------|------|-----------------------------------------------|------------|
| LCG        | 1948 | `state = state × a + c mod 2^64`; upper 32 b | 2^64       |
| Xorshift64 | 2003 | Three XOR-shift operations on 64-bit state    | 2^64 − 1   |
| PCG32      | 2014 | LCG recurrence + XSH RR output permutation   | 2^64       |

Constants (Knuth / Numerical Recipes):

```
LCG_MULTIPLIER = 6364136223846793005
LCG_INCREMENT  = 1442695040888963407
```

## Usage

```haskell
import Rng

-- LCG
let g0        = newLCG 42
    (v1, g1)  = nextU32 g0        -- Word32 in [0, 2^32)
    (v2, g2)  = nextU64 g1        -- Word64 in [0, 2^64)
    (v3, g3)  = nextFloat g2      -- Double in [0.0, 1.0)
    (v4, _)   = nextIntInRange 1 6 g3  -- Int64 in [1, 6] inclusive

-- Xorshift64
let xs        = newXorshift64 42
    (w, xs')  = nextU32 xs

-- PCG32
let p         = newPCG32 42
    (u, p')   = nextU32 p
```

## Reference values (seed = 1)

| Generator  | Output 1   | Output 2   | Output 3   |
|------------|------------|------------|------------|
| LCG        | 1817669548 | 2187888307 | 2784682393 |
| Xorshift64 | 1082269761 |  201397313 | 1854285353 |
| PCG32      | 1412771199 | 1791099446 |  124312908 |

## API

```haskell
class RandomGen g where
    nextU32       :: g -> (Word32, g)
    nextU64       :: g -> (Word64, g)
    nextFloat     :: g -> (Double, g)
    nextIntInRange :: Int64 -> Int64 -> g -> (Int64, g)
```

`nextU64`, `nextFloat`, and `nextIntInRange` have default implementations in
terms of `nextU32`, so a new generator only needs to implement `nextU32`.

## Implementation notes

- Uses `Data.Word.Word64` / `Word32` throughout — these wrap on overflow
  exactly like unsigned integers in C or Go, so no `mod 2^64` masking is
  needed.
- PCG32 warm-up: the "initseq" procedure advances from state=0, adds the
  seed, then advances once more — matching the reference C implementation.
- `nextIntInRange` uses rejection sampling to eliminate modulo bias.

## Running tests

```bash
cabal test
```

Expected: 29 examples, 0 failures.

## Layer

CS03 — leaf package, no external dependencies beyond `base`.

## Spec

`code/specs/CS03-rng.md`
