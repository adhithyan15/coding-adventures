# Trig

Trigonometric functions implemented from first principles, with no `Math` library
calls. Since version 0.3.0 this port follows the shared reference algorithm
**TRIG-REF-1** ([PHY00a](../../../specs/PHY00a-trig-convergence.md)), so `sin`, `cos`,
`tan`, `radians` and `degrees` return exactly the same 64 bits as every other converged
language port.

**Layer:** PHY00 (leaf package, no dependencies)

## How It Works

### Range reduction (Cody-Waite)

`k = (x * 2/pi + MAGIC) - MAGIC` is the nearest whole number of quarter-turns in `x`
(adding `1.5 * 2^52` rounds to an integer using only an add and a subtract). The
remainder `r = x - k * pi/2` is formed by subtracting five short slices of `pi/2`, each
with at most 24 bits, so every product `k * slice` is exact. `|r| <= pi/4`.

### Horner kernels

On the reduced angle, `sin` and `cos` are ten-term Maclaurin polynomials with stored
coefficients `1/3!, 1/5!, ...` and `1/4!, 1/6!, ...`, evaluated by Horner's rule (smallest
terms first, no division). The quadrant `k mod 4` selects `sin r`, `cos r`, `-sin r` or
`-cos r`.

### Domain and special values

| Input | `sin` / `cos` |
|---|---|
| NaN, `+-inf`, `\|x\| > 2^29` | NaN (outside the domain; fails closed) |
| `-0.0` | `sin` is `-0.0`, `cos` is `1.0` |
| `\|x\| < 2^-27` | `sin(x) = x`, `cos(x) = 1.0` |

`tan` is `sin / cos` with the legacy pole sentinel `+-1e308` where `|cos| < 1e-15`;
NaN outside the domain. `radians`/`degrees` are one multiply by a pinned constant.
`atan2` returns NaN if either argument is NaN. `sqrt`, `atan` and `atan2` keep their
earlier algorithms, pinned bit for bit by the same vectors.

### Accuracy

Measured against the correctly rounded value over the shared vectors: `sin` and `cos`
within 3 ulp, `radians` and `degrees` within 1 ulp. No accuracy claim is made for `tan`,
`sqrt`, `atan` or `atan2` (they are pinned, not proven accurate).

## API

| Method | Description |
|---|---|
| `Trig.sin(x)` | Sine of x (radians) |
| `Trig.cos(x)` | Cosine of x (radians) |
| `Trig.tan(x)` | Tangent, `+-1e308` at the poles |
| `Trig.radians(deg)` / `Trig.degrees(rad)` | Unit conversion |
| `Trig.sqrt(x)` | Square root (Newton); `ArgumentError` for negative input |
| `Trig.atan(x)` / `Trig.atan2(y, x)` | Arctangent, four-quadrant arctangent |
| `Trig::PI`, `TWO_PI`, `HALF_PI` | Bit-pinned constants |

Integer arguments are converted once with `to_f`.

## Usage

```ruby
require_relative 'lib/trig'

Trig.sin(Trig::PI / 2)   # => 1.0
Trig.cos(0)              # => 1.0
Trig.radians(180)        # => 3.141592653589793
Trig.degrees(Trig::PI)   # => 180.0
Trig.sin(1e20)           # => NaN (beyond 2^29 radians)
```

## Running Tests

```bash
ruby test/test_trig.rb
ruby test/test_conformance.rb   # bit-exact against code/specs/fixtures/trig-reference-v1
```
