# trig — Kotlin

## Exact small arctangent

`Trig.atan` returns `x` unchanged when `|x| <= 2^-27`, before any half-angle
reduction. This exact binary64 identity preserves negative zero and both signs
of the minimum subnormal; the shared PHY00 fixture locks those boundaries.

## Full-range square root

The square-root API follows PHY00 across the full binary64 range. It preserves
negative zero, returns positive infinity and NaN unchanged, rejects negative
inputs with the lane-native error, normalizes by powers of four into `[0.25, 4)`,
and then applies at most 60 Newton steps without calling the host square-root
routine. Boundary behavior is checked against
[`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json).

Trigonometric functions built from first principles using Maclaurin series.
No `kotlin.math.sin`, `kotlin.math.cos`, or any other standard library math
functions are used internally — every result is derived from addition,
subtraction and multiplication (plus division in `tan` and the Part II
functions).

Since 0.2.0 `sin`, `cos`, `tan`, `radians` and `degrees` follow the reference
algorithm **TRIG-REF-1** of [PHY00a](../../../specs/PHY00a-trig-convergence.md)
*bit for bit*, and `ConformanceTest` checks all 62,154 vectors of
[`trig-reference-v1`](../../../specs/fixtures/trig-reference-v1/README.md).

## What It Implements

| Function | Description |
|----------|-------------|
| `Trig.sin(x)` | Sine (radians), Cody-Waite reduction + 10-term Horner kernel |
| `Trig.cos(x)` | Cosine (radians), same machinery |
| `Trig.tan(x)` | `sin/cos`, with the pinned `+-1e308` pole sentinel |
| `Trig.sqrt(x)` | Square root via Newton's (Babylonian) method |
| `Trig.atan(x)` | Arctangent, Taylor series with half-angle reduction |
| `Trig.atan2(y, x)` | Four-quadrant arctangent (NaN in, NaN out) |
| `Trig.radians(deg)` | Degrees to radians: one multiply by `pi/180` |
| `Trig.degrees(rad)` | Radians to degrees: one multiply by `180/pi` |
| `Trig.PI` | π constant to full double precision |

## Domain and accuracy

- `sin`, `cos` and `tan` are defined for `|x| <= 2^29` (536870912). Outside it,
  and for NaN and the infinities, they return **NaN**: a value that cannot be
  mistaken for a sine. Reduce an accumulated phase (for example in whole cycles)
  before calling.
- `sin(-0.0)` is `-0.0`; `|x| < 2^-27` returns `x` (`sin`) or `1.0` (`cos`).
- `sin` and `cos` are within **3 ulp** of the exact value over the domain
  (observed worst 2.38 ulp); `radians` and `degrees` within 1 ulp. `tan` makes
  no accuracy claim (its pole sentinel is a legacy convention, not an
  approximation). Details: PHY00a section 4.

## How It Works

### Range reduction (nearest-integer Cody-Waite)

```
t = x * (2/pi)                   one rounding
k = (t + 1.5*2^52) - 1.5*2^52    t rounded to the nearest integer, add and subtract only
r = x - k*P1 - k*P2 - k*P3 - k*P4 - k*P5     pi/2 = P1+...+P5; P1..P4 have 24 bits
```

Because `P1..P4` have at most 24 significant bits, `k * Pi` is exact for
`|k| < 2^29`. The quadrant `k and 3` then selects `sin r`, `cos r`, `-sin r` or
`-cos r`.

### Maclaurin series, evaluated by Horner's rule

```
sin(r) = r + r*z*(S1 + S2*z + ... + S9*z^8)          z = r*r, S[n] = (-1)^n/(2n+1)!
cos(r) = 1 - z/2 + z^2*(C2 + C3*z + ... + C9*z^7)    C[n] = (-1)^n/(2n)!
```

The coefficients are stored as exact bit patterns (`Double.fromBits`). Kotlin/JVM
arithmetic is strict IEEE (Java 17+) and this code never calls `Math.fma`, so
the bits are portable.

## Usage

```kotlin
import com.codingadventures.trig.Trig

val s = Trig.sin(Trig.PI / 4)           // ≈ 0.7071...
val c = Trig.cos(Trig.radians(60.0))    // ≈ 0.5
val r = Trig.sqrt(2.0)                   // ≈ 1.41421...
val a = Trig.atan2(1.0, 1.0)            // ≈ π/4
```

## Running Tests

```bash
gradle test
```

## Part of the Coding Adventures series

Kotlin counterpart to the Python, Rust, Go, TypeScript, and Java implementations.
The converged ports (including this one since 0.2.0) return identical bits for
`sin`, `cos`, `tan`, `radians` and `degrees`.
