# coding-adventures-rng (Lua)

Three classic pseudorandom number generators implemented in Lua 5.4.

## Algorithms

| Generator     | Author      | Year | Period      | Output  | Notes                          |
|---------------|-------------|------|-------------|---------|--------------------------------|
| LCG           | Knuth       | 1948 | 2^64        | upper 32 bits | Simplest full-period PRNG  |
| Xorshift64    | Marsaglia   | 2003 | 2^64 − 1    | lower 32 bits | No multiply, 3 XOR-shifts  |
| PCG32         | O'Neill     | 2014 | 2^64        | XSH RR  | Passes all statistical suites  |

All three generators produce identical reference values from seed=1:

```
LCG:        [1817669548, 2187888307, 2784682393]
Xorshift64: [1082269761, 201397313,  1854285353]
PCG32:      [1412771199, 1791099446, 124312908 ]
```

## API

All three generators expose the same four-method API:

```lua
local rng = require("coding_adventures.rng")

-- LCG
local g = rng.LCG.new(42)
local u32   = g:next_u32()               -- integer in [0, 2^32)
local u64   = g:next_u64()               -- 64-bit integer (Lua integer)
local f     = g:next_float()             -- float in [0.0, 1.0)
local die   = g:next_int_in_range(1, 6)  -- integer in [1, 6] inclusive

-- Same API for Xorshift64 and PCG32:
local xs  = rng.Xorshift64.new(42)
local pcg = rng.PCG32.new(42)
```

## Algorithm Details

### LCG (Linear Congruential Generator)

```
state  = (state × 6364136223846793005 + 1442695040888963407) mod 2^64
output = state >> 32   (upper 32 bits)
```

The constants (Knuth / Numerical Recipes) satisfy the Hull-Dobell theorem,
giving the full period of 2^64. The upper bits are output because the lower
bits have shorter sub-periods (the lowest bit simply alternates 0-1).

### Xorshift64

```
x ^= x << 13
x ^= x >> 7
x ^= x << 17
output = lower 32 bits
```

Seed 0 is replaced with 1 (state 0 is a fixed point — 0 XOR 0 = 0 forever).

### PCG32 (Permuted Congruential Generator)

Uses the LCG recurrence plus XSH RR output permutation on the old state:

```
xorshifted = ((old >> 18) ^ old) >> 27
rot        = old >> 59
output     = rotr32(xorshifted, rot)
```

Initialization uses a two-step warm-up (advance, add seed, advance) so even
seed 0 produces a well-distributed sequence.

## Lua Notes

Lua 5.4 integers are 64-bit signed. Arithmetic wraps mod 2^64 automatically
for `*` and `+`. Bitwise operators (`&`, `|`, `~`, `<<`, `>>`) operate on
full 64-bit patterns. For 32-bit output we mask with `& 0xFFFFFFFF`.

`~` is the XOR operator in Lua 5.4 (not bitwise NOT — that is `~x` as unary).
Lua's `>>` is a logical (zero-filling) shift, which is what we need for
unsigned 64-bit values.

## Running Tests

```bash
cd tests && busted . --verbose --pattern=test_
```

Requires [busted](https://luarocks.org/modules/lunarmodules/busted) installed
via LuaRocks. 41 tests covering all three generators.
