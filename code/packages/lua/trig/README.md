# trig

Trigonometric functions computed from first principles. No standard-library trig
function is used: the goal is to understand *how* sine, cosine and tangent work at the
mathematical level.

Since 0.3.0 this port computes the shared reference algorithm **TRIG-REF-1**
([PHY00a](../../../specs/PHY00a-trig-convergence.md)): `sin`, `cos`, `radians` and
`degrees` return exactly the same bits as every other converged port, and the test
suite checks 62,154 shared vectors
([`trig-reference-v1`](../../../specs/fixtures/trig-reference-v1/README.md)) bit for bit.

## What's Inside

| Function | Description |
|---|---|
| `trig.sin(x)` | Sine of x (radians); NaN for NaN, infinities and `abs(x) > 2^29` |
| `trig.cos(x)` | Cosine of x (radians); same domain |
| `trig.tan(x)` | `sin(x) / cos(x)`, with a pole sentinel of `+-1e308` where `abs(cos) < 1e-15` |
| `trig.radians(deg)` | `deg * RAD_PER_DEG` (one multiply) |
| `trig.degrees(rad)` | `rad * DEG_PER_RAD` (one multiply) |
| `trig.sqrt(x)` | Newton's method; errors on negative input |
| `trig.atan(x)`, `trig.atan2(y, x)` | Arctangent (series); `atan2` of a NaN is NaN |
| `trig.PI`, `TWO_PI`, `HALF_PI` | Constants, built from their bit patterns |
| `trig.INV_PIO2`, `PIO2`, `MAGIC`, `LIMIT`, `TINY`, `S`, `C`, `RAD_PER_DEG`, `DEG_PER_RAD` | The reference constants of PHY00a 3.2 |

## How It Works

**Range reduction (Cody-Waite).** Any angle is split into whole quarter-turns plus a
small leftover: `x = k*(pi/2) + r` with `abs(r) <= pi/4`. The integer `k` is the nearest
integer to `x * (2/pi)`, found with the add-and-subtract trick
`(t + MAGIC) - MAGIC` (no `floor`, no `round`, no float-to-integer cast). `pi/2` is stored
as five pieces, the first four with at most 24 bits, so `k * piece` is exact for
`abs(k) < 2^29`; subtracting the five products leaves `r` accurate to a few ulps. The
quadrant `k mod 4` (`math.tointeger(k) & 3`) picks `sin r`, `cos r`, `-sin r` or `-cos r`.

**Series (Horner).** On `abs(r) <= pi/4` the Maclaurin series is evaluated by Horner's
rule on stored coefficients `1/3!, 1/5!, ...`, ten terms each, with no division:

```
sin(r) = r + r*z*(S1 + S2*z + ... + S9*z^8)        z = r*r
cos(r) = (1 - z/2) + z^2*(C2 + C3*z + ... + C9*z^7)
```

**Domain.** `sin`, `cos` and `tan` are defined for `abs(x) <= 2^29` (about 5.4e8 rad).
Outside it (and for NaN and the infinities) they return NaN rather than a plausible but
wrong number. Code that accumulates a phase should keep only the fractional cycle
(`phase - math.floor(phase)`) before multiplying by `2*pi`.

**Accuracy.** Gated against the correctly rounded value: `sin` and `cos` within 3 ulp
(worst observed 2.38), `radians` and `degrees` within 1 ulp. No accuracy claim is made for
`tan`, `sqrt`, `atan` or `atan2` (PHY00a 3.8, 3.9).

**Lua 5.4 integers.** Lua has an integer subtype whose arithmetic wraps, so every value in
the algorithm is a float: constants are floats (built with
`string.unpack("<d", string.pack("<i8", bits))`) and an integer argument is converted
once at entry with `x * 1.0` (which, unlike `x + 0.0`, keeps `-0.0`).

**Special values.** `sin(-0.0)` is `-0.0`; `abs(x) < 2^-27` returns `x` (`sin`) or `1.0`
(`cos`) exactly.

## How It Fits in the Stack

This package is part of the **coding-adventures** monorepo, which builds the computing
stack from transistors to operating systems. `trig` sits at the mathematical foundations
layer, providing functions that would normally come from a standard library but are
implemented here from scratch. In Lua it is used by `affine2d`, `arc2d`, `point2d`,
`bezier2d` and `wave`.

## Usage

```lua
local trig = require("coding_adventures.trig")

print(trig.sin(trig.PI / 6))      -- 0.5 (to within an ulp or two)
print(trig.cos(trig.PI / 3))      -- 0.5
print(trig.tan(trig.PI / 4))      -- 1.0
print(trig.sin(1e20))             -- nan: outside the domain

local rad = trig.radians(90)      -- pi/2
local deg = trig.degrees(trig.PI) -- 180.0
```

## Previous behaviour kept

`atan(x)` returns `x` unchanged for `abs(x) <= 2^-27` (preserving `-0.0` and
subnormals). `sqrt` preserves negative zero, returns `+inf` and NaN unchanged, rejects
negative input with an error, and normalises by powers of four before at most 60 Newton
steps (boundaries checked against
[`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json)).

## Development

```bash
# Run tests (from the package root)
bash BUILD
```

`tests/test_conformance.lua` reads the shared fixture from `code/specs`.

## Dependencies

None. Pure Lua 5.4, no external libraries required.

Test framework: [busted](https://lunarmodules.github.io/busted/)
