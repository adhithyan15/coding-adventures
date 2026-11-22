# trig

## Exact small arctangent

`Atan` returns `x` unchanged when `|x| <= 2^-27`, before any half-angle
reduction. This exact binary64 identity preserves negative zero and both signs
of the minimum subnormal; the shared PHY00 fixture locks those boundaries.

## Full-range square root

The square-root API follows PHY00 across the full binary64 range. It preserves
negative zero, returns positive infinity and NaN unchanged, rejects negative
inputs with the lane-native error, normalizes by powers of four into `[0.25, 4)`,
and then applies at most 60 Newton steps without calling the host square-root
routine. Boundary behavior is checked against
[`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json).

Trigonometric functions built from first principles using Taylor/Maclaurin series.

## Layer

PHY00 — a leaf package with no dependencies.

## What It Does

This package implements `Sin`, `Cos`, `Tan`, `Radians`, `Degrees`, `Sqrt`, `Atan` and
`Atan2` without relying on Go's `math` library. It is the Go port of the shared
reference algorithm TRIG-REF-1 (`code/specs/PHY00a-trig-convergence.md`): every
language's `trig` returns the same bits, and the conformance vectors in
`code/specs/fixtures/trig-reference-v1` check that bit for bit.

The source code is written in literate programming style: reading it should teach you how
trigonometric functions work under the hood.

## API

| Function       | Description                                                  |
|----------------|--------------------------------------------------------------|
| `Sin(x)`       | Sine of x (radians); NaN outside `\|x\| <= 2^29`            |
| `Cos(x)`       | Cosine of x (radians); NaN outside `\|x\| <= 2^29`          |
| `Tan(x)`       | `Sin/Cos`, with a `+-1e308` sentinel at the poles; no accuracy claim |
| `Radians(deg)` | One multiply by `pi/180`                                     |
| `Degrees(rad)` | One multiply by `180/pi`                                     |
| `Sqrt(x)`      | Newton's method; panics for negative inputs                  |
| `Atan(x)`      | Arctangent via Taylor series with range reduction            |
| `Atan2(y, x)`  | Four-quadrant arctangent; NaN if an argument is NaN          |

### Constants

| Constant | Value              |
|----------|--------------------|
| `PI`     | 3.141592653589793  |
| `TwoPI`  | 6.283185307179586  |

## How It Works

1. **Domain guard**: `|x| <= 2^29` or the result is NaN (one negated comparison that
   also catches NaN and infinities). Below `2^-27` `Sin(x) = x` and `Cos(x) = 1`.
2. **Range reduction (Cody-Waite)**: `k = round(x * 2/pi)` using the
   `(t + 1.5*2^52) - 1.5*2^52` trick, then `r = x - k*pi/2` with `pi/2` stored as five
   pieces, the first four short enough that `k * piece` is exact.
3. **Quadrant**: `k & 3` selects `sin(r)`, `cos(r)`, `-sin(r)` or `-cos(r)`.
4. **Horner kernels**: ten Maclaurin terms with stored coefficients `1/3!, 1/5!, ...`;
   no division anywhere.

Accuracy against the correctly rounded value, on `|x| <= 2^29`: `Sin` and `Cos` within
3 ulp (observed 2.4), `Radians` and `Degrees` within 1 ulp.

### No fused multiply-add

Go may fuse `x*y + z` on arm64 and other targets and has no flag to stop it. Every such
product is written `float64(x*y)` (an explicit conversion always rounds), which makes
the results identical on every architecture. `BUILD` checks that the arm64 and
`GOAMD64=v3` assembly contains no fused instructions. Do not remove the conversions.

## Usage

```go
package main

import (
    "fmt"
    "github.com/adhithyan15/coding-adventures/code/packages/go/trig"
)

func main() {
    fmt.Println(trig.Sin(trig.PI / 6))  // 0.5
    fmt.Println(trig.Cos(0))             // 1.0
    fmt.Println(trig.Radians(180))       // 3.141592653589793
    fmt.Println(trig.Degrees(trig.PI))   // 180.0
}
```
