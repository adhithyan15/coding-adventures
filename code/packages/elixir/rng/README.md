# coding_adventures_rng (Elixir)

Three classic pseudorandom number generators implemented in pure Elixir,
matching the Go reference implementation bit-for-bit.

## Algorithms

### LCG — Linear Congruential Generator (Knuth 1948)

The simplest useful PRNG.  State advances via:

```
state = (state × a + c) mod 2^64
```

where `a = 6_364_136_223_846_793_005` and `c = 1_442_695_040_888_963_407`
(Knuth / Numerical Recipes constants).  These satisfy the Hull-Dobell theorem,
giving a full period of 2^64.  Output is the upper 32 bits of state.

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

Add to your `mix.exs`:

```elixir
defp deps do
  [
    {:coding_adventures_rng, "~> 0.1"}
  ]
end
```

## Usage

All generators are pure-functional structs.  Every function takes a generator
and returns `{value, new_generator}` — no mutation, no processes.

```elixir
alias CodingAdventures.Rng.LCG
alias CodingAdventures.Rng.Xorshift64
alias CodingAdventures.Rng.PCG32

# Create a seeded generator (returns {:ok, generator})
{:ok, g} = LCG.new(42)         # or Xorshift64.new(42) / PCG32.new(42)

{v, g} = LCG.next_u32(g)               # v  in [0, 2^32)
{u, g} = LCG.next_u64(g)               # u  in [0, 2^64)
{f, g} = LCG.next_float(g)             # f  in [0.0, 1.0)
{n, g} = LCG.next_int_in_range(g, 1, 6) # n in [1, 6] inclusive (unbiased)
```

### Reference values (seed = 1)

| Generator  | Call 1       | Call 2       | Call 3       |
|------------|-------------|-------------|-------------|
| LCG        | 1817669548  | 2187888307  | 2784682393  |
| Xorshift64 | 1082269761  | 201397313   | 1854285353  |
| PCG32      | 1412771199  | 1791099446  | 124312908   |

### Composing generators in pipelines

Because every function returns `{value, new_generator}`, generators compose
naturally with `Enum.map_reduce/3`:

```elixir
{:ok, g} = PCG32.new(42)
{values, _g} = Enum.map_reduce(1..10, g, fn _, acc ->
  PCG32.next_int_in_range(acc, 1, 100)
end)
# => [list of 10 random integers in [1, 100]]
```

### Elixir-specific notes

- All three modules `use CodingAdventures.Rng` to inherit shared constants
  (`@multiplier`, `@increment`, `@mask64`, `@mask32`, `@float_div`) via the
  `__using__` macro.
- `import Bitwise` is used (not the deprecated `use Bitwise`).
- All arithmetic is masked with `band(x, @mask64)` after each multiply/add to
  simulate unsigned 64-bit overflow (Elixir integers are arbitrary precision).

## Development

```bash
# Fetch dependencies (none required for production)
mix deps.get

# Run tests with coverage
mix test --cover

# Expected output: 38 tests, 0 failures, ~94% total coverage
```

## How it fits in the stack

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
monorepo — a ground-up tour of the computing stack from transistors to
operating systems.  The same three PRNG algorithms are implemented in every
supported language with identical reference values so correctness is
cross-verified across implementations.
