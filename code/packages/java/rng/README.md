# rng (Java)

Three classic pseudorandom number generators implemented in Java, cross-compatible with the Go reference implementation.

## Algorithms

| Class | Period | Notes |
|-------|--------|-------|
| `Rng.Lcg` | 2^64 | Linear Congruential Generator (Knuth 1948). Fast full-period PRNG; consecutive outputs are linearly correlated. |
| `Rng.Xorshift64` | 2^64 − 1 | Marsaglia (2003) XOR-shift. No multiplication; seed 0 replaced with 1. |
| `Rng.Pcg32` | 2^64 | Permuted Congruential Generator (O'Neill 2014). LCG state + XSH RR output permutation. Passes all known statistical test suites. |

All three generators share the same API surface:

```java
// Factory method — any seed is valid
Rng.Lcg lcg = Rng.Lcg.of(42L);

long   u32   = lcg.nextU32();              // uniform value in [0, 2^32)
long   u64   = lcg.nextU64();              // (hi << 32) | lo from two nextU32 calls
double f     = lcg.nextFloat();            // uniform double in [0.0, 1.0)
long   roll  = lcg.nextIntInRange(1, 6);   // uniform long in [min, max] inclusive
```

## Constants

```
LCG_MULTIPLIER = 6364136223846793005  (0x5851F42D4C957F2D)
LCG_INCREMENT  = 1442695040888963407  (0x14057B7EF767814F)
```

These are the Knuth / Numerical Recipes constants.  Together they satisfy
the Hull-Dobell theorem, giving LCG a full period of 2^64.

## Known Reference Values (seed = 1)

| Generator | Call 1 | Call 2 | Call 3 |
|-----------|--------|--------|--------|
| LCG | 1817669548 | 2187888307 | 2784682393 |
| Xorshift64 | 1082269761 | 201397313 | 1854285353 |
| PCG32 | 1412771199 | 1791099446 | 124312908 |

## Implementation Notes

- Java `long` is signed 64-bit, but two's-complement overflow wraps exactly
  as needed for mod-2^64 arithmetic — no explicit masking for multiply/add.
- `>>>` (unsigned right shift) is used wherever bits are treated as unsigned.
- `nextU32()` returns `long` (not `int`) to expose the full unsigned range
  [0, 2^32) without sign-extension surprises.
- `nextIntInRange` uses `Long.remainderUnsigned` and `Long.compareUnsigned`
  for correct threshold arithmetic.

## Testing

27 JUnit 5 tests covering reference values, reproducibility, range bounds,
`nextU64` consistency, `nextFloat` distribution, rejection-sampling bounds,
edge cases (seed 0, large seeds), and PCG32 initseq warm-up.

```bash
gradle test
```

## Development

```bash
# Run tests
bash BUILD
```
