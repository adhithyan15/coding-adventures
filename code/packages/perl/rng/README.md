# CodingAdventures::Rng (Perl)

Three classic pseudorandom number generators implemented in pure Perl.

## Algorithms

| Class                              | Author    | Year | Period    | Output  |
|------------------------------------|-----------|------|-----------|---------|
| `CodingAdventures::Rng::LCG`       | Knuth     | 1948 | 2^64      | upper 32 bits |
| `CodingAdventures::Rng::Xorshift64`| Marsaglia | 2003 | 2^64 − 1  | lower 32 bits |
| `CodingAdventures::Rng::PCG32`     | O'Neill   | 2014 | 2^64      | XSH RR  |

All three produce identical reference values from seed=1:

```
LCG:        [1817669548, 2187888307, 2784682393]
Xorshift64: [1082269761, 201397313,  1854285353]
PCG32:      [1412771199, 1791099446, 124312908 ]
```

## API

All three classes expose the same four-method API:

```perl
use CodingAdventures::Rng;

my $g = CodingAdventures::Rng::LCG->new(42);
my $u32 = $g->next_u32();               # integer in [0, 2^32)
my $u64 = $g->next_u64();               # Math::BigInt
my $f   = $g->next_float();             # float in [0.0, 1.0)
my $die = $g->next_int_in_range(1, 6);  # integer in [1, 6] inclusive

# Same API for Xorshift64 and PCG32:
my $xs  = CodingAdventures::Rng::Xorshift64->new(42);
my $pcg = CodingAdventures::Rng::PCG32->new(42);
```

## Algorithm Details

### LCG (Linear Congruential Generator)

```
state  = (state × 6364136223846793005 + 1442695040888963407) mod 2^64
output = state >> 32   (upper 32 bits)
```

### Xorshift64

```
x ^= x << 13;  x ^= x >> 7;  x ^= x << 17
output = lower 32 bits
```

Seed 0 is replaced with 1.

### PCG32 (Permuted Congruential Generator)

LCG recurrence plus XSH RR output permutation on old state.
Two-step initseq warm-up ensures good distribution even for seed 0.

## Perl Implementation Notes

Perl uses IEEE 754 doubles by default, which have only 53 bits of mantissa.
This is insufficient for 64-bit integer multiplication. We use `Math::BigInt`
for the LCG multiply step and for the Xorshift64 shift operations to avoid
precision loss.

Key gotcha: `Math::BigInt` methods (`bxor`, `band`, `blsft`, `brsft`) modify
objects IN PLACE and return the same reference. Always work on `.copy()` when
you need the original value preserved.

`next_u32` returns a plain Perl native integer (UV). `next_u64` returns a
`Math::BigInt` object.

## Dependencies

- `Math::BigInt` (core module, no installation needed)
- `Test2::V0` (test dependency only)

## Running Tests

```bash
prove -l -v t/
```

59 tests across 5 test files covering all three generators.
