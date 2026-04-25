# coding_adventures_rng (Ruby)

Three classic pseudorandom number generators implemented in pure Ruby, matching
the Go reference implementation bit-for-bit.

## Algorithms

### LCG — Linear Congruential Generator (Knuth 1948)

The simplest useful PRNG.  State advances via:

```
state = (state × a + c) mod 2^64
```

where `a = 6_364_136_223_846_793_005` and `c = 1_442_695_040_888_963_407`
(Knuth / Numerical Recipes constants).  These satisfy the Hull-Dobell theorem,
giving a full period of 2^64.  Output is the upper 32 bits of state.

Strengths: extremely fast, no branching, full period.
Weaknesses: consecutive outputs are linearly correlated; fails spectral tests
in high dimensions.

### Xorshift64 — Marsaglia (2003)

Three XOR-shift operations scramble 64-bit state without any multiplication:

```
x ^= x << 13
x ^= x >> 7
x ^= x << 17
```

Period: 2^64 − 1.  State 0 is a fixed point and is replaced with 1.
Output is the lower 32 bits of state.

### PCG32 — Permuted Congruential Generator (O'Neill 2014)

Uses the same LCG recurrence but applies an XSH RR output permutation before
returning bits.  The permutation (XOR-Shift High / Random Rotate) breaks the
linear correlation that makes plain LCG weak.

Passes all known statistical test suites (TestU01 BigCrush, PractRand) with
only 8 bytes of state.

## Installation

Add to your `Gemfile`:

```ruby
gem "coding_adventures_rng"
```

Or install directly:

```bash
gem install coding_adventures_rng
```

## Usage

All three generators expose an identical API:

```ruby
require "coding_adventures_rng"

# Create a seeded generator
g = CodingAdventures::Rng::LCG.new(42)
# or
g = CodingAdventures::Rng::Xorshift64.new(42)
# or
g = CodingAdventures::Rng::PCG32.new(42)

g.next_u32                  # => Integer in [0, 2^32)
g.next_u64                  # => Integer in [0, 2^64)
g.next_float                # => Float   in [0.0, 1.0)
g.next_int_in_range(1, 6)   # => Integer in [1, 6] inclusive (unbiased)
```

### Reference values (seed = 1)

| Generator  | Call 1       | Call 2       | Call 3       |
|------------|-------------|-------------|-------------|
| LCG        | 1817669548  | 2187888307  | 2784682393  |
| Xorshift64 | 1082269761  | 201397313   | 1854285353  |
| PCG32      | 1412771199  | 1791099446  | 124312908   |

### Rejection sampling

`next_int_in_range` uses rejection sampling to eliminate modulo bias.  Naive
`value % range` over-samples low values when 2^32 is not divisible by the
range size.  The threshold is computed as:

```ruby
threshold = (-range_size) % (1 << 32) % range_size
```

Any draw below `threshold` is discarded.  Expected extra iterations per call
is less than 2 for all range sizes.

### Ruby-specific notes

Ruby integers are arbitrary precision, so every multiply or add that simulates
unsigned 64-bit arithmetic must be masked:

```ruby
MASK64 = 0xFFFF_FFFF_FFFF_FFFF
state = (state * LCG_MULTIPLIER + LCG_INCREMENT) & MASK64
```

## Development

```bash
# Install dependencies
bundle install

# Run tests with coverage
bundle exec rake test

# Expected output: 38 tests, 0 failures, 100% line coverage
```

## How it fits in the stack

This gem is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
monorepo — a ground-up tour of the computing stack from transistors to
operating systems.  The same three PRNG algorithms are implemented in every
supported language with identical reference values so correctness is
cross-verified across implementations.
