# rng (TypeScript)

Pseudorandom number generator library implementing LCG, Xorshift64, and PCG32.
Output values are identical to the Go reference implementation for the same seed.

## Algorithms

### LCG — Linear Congruential Generator (Knuth 1948)
`state = (state × 6364136223846793005n + 1442695040888963407n) mod 2^64`
Output: upper 32 bits. Full period 2^64.

### Xorshift64 (Marsaglia 2003)
Three XOR-shifts on 64-bit state. Period 2^64−1. Seed 0 → replaced with 1.
Output: lower 32 bits.

### PCG32 (O'Neill 2014)
LCG + XSH RR permutation (XOR-Shift High / Random Rotate). Passes all known
statistical test suites (TestU01 BigCrush, PractRand).

## Usage

```typescript
import { LCG, Xorshift64, PCG32 } from "@coding-adventures/rng";

// Seeds must be passed as BigInt
const g = new PCG32(42n);
const v: number = g.nextU32();              // number in [0, 2^32)
const u: bigint = g.nextU64();              // bigint in [0, 2^64)
const f: number = g.nextFloat();            // number in [0.0, 1.0)
const n: number = g.nextIntInRange(1, 6);   // die roll, inclusive
```

## Known Reference Values (seed=1n)

| Call | LCG        | Xorshift64 | PCG32      |
|------|------------|------------|------------|
| 1st  | 1817669548 | 1082269761 | 1412771199 |
| 2nd  | 2187888307 | 201397313  | 1791099446 |
| 3rd  | 2784682393 | 1854285353 | 124312908  |

## Implementation Notes

JavaScript `number` is a 64-bit IEEE 754 double with only 53 bits of integer
precision — not enough to hold 64-bit state. All internal state arithmetic
uses `BigInt` with `& MASK64` (`0xFFFFFFFFFFFFFFFFn`) to emulate unsigned
64-bit wrapping. `nextU32`, `nextFloat`, and `nextIntInRange` return `number`;
`nextU64` returns `bigint`. Seeds must be passed as `bigint` literals (e.g.
`42n`).

## Development

```bash
npm test
# or via build-tool
bash BUILD
```

Coverage: 31 tests.
