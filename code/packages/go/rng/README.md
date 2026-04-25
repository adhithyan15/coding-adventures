# rng (Go)

Pseudorandom number generator library implementing LCG, Xorshift64, and PCG32.
This is the **reference implementation** — the known output values it produces
for seed=1 are the ground truth that all other language implementations must match.

## Algorithms

### LCG — Linear Congruential Generator (Knuth 1948)
`state = (state × 6364136223846793005 + 1442695040888963407) mod 2^64`  
Output: upper 32 bits. Full period 2^64.

### Xorshift64 (Marsaglia 2003)
Three XOR-shifts on 64-bit state. Period 2^64−1. Seed 0 → replaced with 1.  
Output: lower 32 bits.

### PCG32 (O'Neill 2014)
LCG + XSH RR permutation. Passes all known statistical test suites.

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/rng"

g := rng.NewPCG32(42)
v := g.NextU32()            // uint32
f := g.NextFloat()          // float64 in [0.0, 1.0)
n := g.NextIntInRange(1, 6) // die roll
```

## Known Reference Values (seed=1)

| Call | LCG | Xorshift64 | PCG32 |
|------|-----|------------|-------|
| 1st | 1817669548 | 1082269761 | 1412771199 |
| 2nd | 2187888307 | 201397313 | 1791099446 |
| 3rd | 2784682393 | 1854285353 | 124312908 |

## Development

```bash
bash BUILD
# or
go test -cover ./...
```

Coverage: 100%.
