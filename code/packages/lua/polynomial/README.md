# coding-adventures-polynomial (Lua)

Coefficient-array polynomial arithmetic over real numbers — part of the
`coding-adventures` math stack (layer MA00).

## What It Does

This package represents polynomials as Lua arrays where **index k+1 holds
the coefficient of x^k** (little-endian / lowest-degree first):

```lua
{3, 0, 2}   -- 3 + 0·x + 2·x²
{1, 2}      -- 1 + 2x
{0}         -- the zero polynomial
```

All operations return **normalized** polynomials — trailing near-zero
coefficients are stripped, so `{1, 0, 0}` and `{1}` are both represented
as `{1}`.

## Stack Position

```
MA02 — reed-solomon  (uses polynomial + gf256)
MA01 — gf256         (uses polynomial for modular reduction)
MA00 — polynomial    ← you are here
```

## Quick Start

```lua
local poly = require("coding_adventures.polynomial")

-- Build polynomials
local a = {1, 2, 3}   -- 1 + 2x + 3x²
local b = {4, 5}      -- 4 + 5x

-- Basic operations
print(poly.degree(a))           -- 2
print(poly.evaluate(a, 2))      -- 1 + 4 + 12 = 17

local sum  = poly.add(a, b)     -- {5, 7, 3}
local diff = poly.subtract(a, b)-- {-3, -3, 3}
local prod = poly.multiply(a, b)-- convolution

-- Division
local q, r = poly.divmod({5,1,3,2}, {2,1})
-- q = {3,-1,2}, r = {-1}
-- Verify: (2+x)(3-x+2x²) + (-1) = 5+x+3x²+2x³  ✓

-- Evaluation via Horner's method
local val = poly.evaluate({3, 1, 2}, 4)   -- 39

-- GCD
local g = poly.gcd({2,3,1}, {3,4,1})
-- Greatest common divisor of (x+1)(x+2) and (x+1)(x+3) → proportional to (x+1)
```

## API Reference

| Function | Description |
|----------|-------------|
| `normalize(p)` | Strip trailing near-zero coefficients; always returns at least `{0}` |
| `degree(p)` | Highest non-zero degree; -1 for the zero polynomial |
| `zero()` | Returns `{0}` — the additive identity |
| `one()` | Returns `{1}` — the multiplicative identity |
| `add(a, b)` | Term-by-term addition |
| `subtract(a, b)` | Term-by-term subtraction |
| `multiply(a, b)` | Polynomial convolution |
| `divmod(a, b)` | Long division → `(quotient, remainder)`; errors if `b` is zero |
| `divide(a, b)` | Quotient only |
| `modulo(a, b)` | Remainder only |
| `evaluate(p, x)` | Horner evaluation at point `x` |
| `gcd(a, b)` | Euclidean GCD |

## Running Tests

```bash
cd tests && busted . --verbose --pattern=test_
```

Requires [busted](https://olivinelabs.com/busted/) and Lua 5.4+.

## Dependencies

- Lua ≥ 5.4
- No external Lua dependencies

## License

MIT
