# rng (Python)

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

```python
from rng import LCG, Xorshift64, PCG32

g = PCG32(42)
v = g.next_u32()              # int in [0, 2^32)
u = g.next_u64()              # int in [0, 2^64)
f = g.next_float()            # float in [0.0, 1.0)
n = g.next_int_in_range(1, 6) # die roll, inclusive
```

## Known Reference Values (seed=1)

| Call | LCG        | Xorshift64 | PCG32      |
|------|------------|------------|------------|
| 1st  | 1817669548 | 1082269761 | 1412771199 |
| 2nd  | 2187888307 | 201397313  | 1791099446 |
| 3rd  | 2784682393 | 1854285353 | 124312908  |

## Implementation Notes

Python integers are arbitrary precision. Every multiply and add is masked
with `& _MASK64` (`(1 << 64) - 1`) to emulate 64-bit unsigned wrapping.
32-bit output is masked with `& _MASK32`. The PCG32 rotate-right is computed
as `(x >> rot) | ((x << ((-rot) & 31)) & _MASK32)`.

## Development

```bash
pip install -e ".[dev]"
pytest tests/
# or via build-tool
bash BUILD
```

Coverage: 100% (36 tests).
