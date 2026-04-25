# rng (Rust)

Pseudorandom number generator library implementing LCG, Xorshift64, and PCG32.
Output values are identical to the Go reference implementation for the same seed.

## Algorithms

### LCG — Linear Congruential Generator (Knuth 1948)
`state = (state × 6364136223846793005 + 1442695040888963407) mod 2^64`
Output: upper 32 bits. Full period 2^64.

### Xorshift64 (Marsaglia 2003)
Three XOR-shifts on 64-bit state. Period 2^64−1. Seed 0 → replaced with 1.
Output: lower 32 bits.

### PCG32 (O'Neill 2014)
LCG + XSH RR permutation (XOR-Shift High / Random Rotate). Passes all known
statistical test suites (TestU01 BigCrush, PractRand).

## Usage

```rust
use rng::{Lcg, Xorshift64, Pcg32};

let mut g = Pcg32::new(42);
let v: u32  = g.next_u32();              // uint32 in [0, 2^32)
let u: u64  = g.next_u64();              // uint64 in [0, 2^64)
let f: f64  = g.next_float();            // float64 in [0.0, 1.0)
let n: i64  = g.next_int_in_range(1, 6); // die roll, inclusive
```

## Known Reference Values (seed=1)

| Call | LCG        | Xorshift64 | PCG32      |
|------|------------|------------|------------|
| 1st  | 1817669548 | 1082269761 | 1412771199 |
| 2nd  | 2187888307 | 201397313  | 1791099446 |
| 3rd  | 2784682393 | 1854285353 | 124312908  |

## Implementation Notes

Rust integers panic on overflow in debug mode. All 64-bit arithmetic uses
`wrapping_mul` / `wrapping_add` to match C-style modular semantics exactly.
The PCG32 rotate uses `u32::rotate_right` which maps to a single `ROR`
instruction on x86-64.

## Development

```bash
cargo test
# or via build-tool
bash BUILD
```

Coverage: 100% (27 unit tests + 1 doc test).
