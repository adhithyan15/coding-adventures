# rng (Swift)

Three classic pseudorandom number generators implemented as Swift value types:
**LCG** (Knuth 1948), **Xorshift64** (Marsaglia 2003), and **PCG32** (O'Neill 2014).

All three are `struct` types that expose the same five-method interface.
Swift's overflow arithmetic operators (`&*`, `&+`, `&>>`, `&<<`) are used
throughout so `UInt64`/`UInt32` wrap correctly without explicit masking.

## Algorithms

| Generator  | Year | Core idea                                     | Period     |
|------------|------|-----------------------------------------------|------------|
| LCG        | 1948 | `state = state &* a &+ c`; upper 32 bits out  | 2^64       |
| Xorshift64 | 2003 | Three XOR-shift operations on 64-bit state    | 2^64 − 1   |
| PCG32      | 2014 | LCG recurrence + XSH RR output permutation   | 2^64       |

Constants (Knuth / Numerical Recipes):

```
LCG_MULTIPLIER = 6364136223846793005
LCG_INCREMENT  = 1442695040888963407
```

## Usage

```swift
import Rng

// LCG
var lcg = LCG(seed: 42)
let v1  = lcg.nextU32()              // UInt32 in [0, 2^32)
let v2  = lcg.nextU64()              // UInt64 in [0, 2^64)
let v3  = lcg.nextFloat()            // Double in [0.0, 1.0)
let v4  = lcg.nextIntInRange(min: 1, max: 6)  // Int64 in [1, 6]

// Xorshift64
var xs  = Xorshift64(seed: 42)
let w   = xs.nextU32()

// PCG32
var pcg = PCG32(seed: 42)
let u   = pcg.nextU32()
```

## Reference values (seed = 1)

| Generator  | Output 1   | Output 2   | Output 3   |
|------------|------------|------------|------------|
| LCG        | 1817669548 | 2187888307 | 2784682393 |
| Xorshift64 | 1082269761 |  201397313 | 1854285353 |
| PCG32      | 1412771199 | 1791099446 |  124312908 |

## API

All three types conform to the same interface:

```swift
init(seed: UInt64)
mutating func nextU32() -> UInt32
mutating func nextU64() -> UInt64
mutating func nextFloat() -> Double
mutating func nextIntInRange(min: Int64, max: Int64) -> Int64
```

Methods are `mutating` because generators are `struct` value types — state
is stored by value and must be mutated explicitly.

## Implementation notes

- `UInt64` arithmetic wraps automatically on overflow — no masking needed.
- PCG32 warm-up: "initseq" procedure advances from state=0, adds seed, then
  advances once more — matches the reference C implementation.
- `nextIntInRange` uses rejection sampling (threshold = `(-range) % range`)
  to eliminate modulo bias.
- PCG32 rotation uses `&>>` / `&<<` overflow shift operators:
  `(x &>> rot) | (x &<< (32 &- rot))`.

## Running tests

```bash
swift test
```

Expected: 29 tests, 0 failures.

## Layer

CS03 — leaf package, no dependencies.

## Spec

`code/specs/CS03-rng.md`
