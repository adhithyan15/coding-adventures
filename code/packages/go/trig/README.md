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

This package implements `Sin`, `Cos`, `Radians`, and `Degrees` without relying on any math library. Every function is computed from scratch using the Maclaurin series expansion, with range reduction to ensure accuracy for all inputs.

The source code is written in literate programming style — reading it should teach you how trigonometric functions actually work under the hood.

## API

| Function       | Description                                      |
|----------------|--------------------------------------------------|
| `Sin(x)`       | Sine of x (radians) via 20-term Maclaurin series |
| `Cos(x)`       | Cosine of x (radians) via 20-term Maclaurin series |
| `Radians(deg)` | Convert degrees to radians                       |
| `Degrees(rad)` | Convert radians to degrees                       |

### Constants

| Constant | Value              |
|----------|--------------------|
| `PI`     | 3.141592653589793  |
| `TwoPI`  | 6.283185307179586  |

## How It Works

1. **Range reduction**: Any input angle is normalized to [-pi, pi] by removing full 2*pi rotations. This ensures the Maclaurin series converges quickly.

2. **Maclaurin series**: Each term is computed iteratively from the previous term (avoiding large factorial/power computations):
   - Sin: `term_n = term_{n-1} * (-x^2) / ((2n)(2n+1))`
   - Cos: `term_n = term_{n-1} * (-x^2) / ((2n-1)(2n))`

3. **20 terms**: More than sufficient for double-precision (float64) accuracy.

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
