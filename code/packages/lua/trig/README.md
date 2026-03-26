# trig

Trigonometric functions computed from first principles using Maclaurin series. No standard-library trig functions are used in the implementation -- the goal is to understand *how* sine, cosine, and tangent work at the mathematical level.

This is a Lua 5.4 port of the Go implementation at `code/packages/go/trig/`.

## What's Inside

| Function | Description |
|---|---|
| `trig.sin(x)` | Sine of x (radians) via 20-term Maclaurin series |
| `trig.cos(x)` | Cosine of x (radians) via 20-term Maclaurin series |
| `trig.tan(x)` | Tangent of x, defined as sin(x)/cos(x) |
| `trig.radians(deg)` | Convert degrees to radians |
| `trig.degrees(rad)` | Convert radians to degrees |
| `trig.PI` | Pi to double precision |
| `trig.TWO_PI` | 2 * Pi |

## How It Works

The core technique is the **Maclaurin series** (a Taylor series centred at zero):

```
sin(x) = x - x^3/3! + x^5/5! - x^7/7! + ...
cos(x) = 1 - x^2/2! + x^4/4! - x^6/6! + ...
```

Each term is computed iteratively from the previous one to avoid factorial overflow:

```
sin term_n = term_{n-1} * (-x^2) / ((2n)(2n+1))
cos term_n = term_{n-1} * (-x^2) / ((2n-1)(2n))
```

**Range reduction** normalises any input angle to [-pi, pi] before evaluating the series, since sin and cos are periodic with period 2*pi. This ensures fast convergence and numerical stability even for large inputs.

## How It Fits in the Stack

This package is part of the **coding-adventures** monorepo, which builds the computing stack from transistors to operating systems. The trig package sits at the mathematical foundations layer, providing functions that would typically come from a standard library but are implemented here from scratch for educational purposes.

## Usage

```lua
local trig = require("coding_adventures.trig")

-- Sine and cosine
print(trig.sin(trig.PI / 6))   -- 0.5
print(trig.cos(trig.PI / 3))   -- 0.5

-- Tangent
print(trig.tan(trig.PI / 4))   -- 1.0

-- Angle conversion
local rad = trig.radians(90)   -- pi/2
local deg = trig.degrees(trig.PI) -- 180
```

## Development

```bash
# Run tests (from the package root)
bash BUILD
```

## Dependencies

None. Pure Lua 5.4, no external libraries required.

Test framework: [busted](https://lunarmodules.github.io/busted/)
