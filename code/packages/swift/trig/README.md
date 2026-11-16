# trig — Trigonometric Functions from First Principles (Swift)

## Converged on TRIG-REF-1 (0.2.0)

`sin`, `cos`, `radians` and `degrees` follow the reference algorithm of
[PHY00a](../../../specs/PHY00a-trig-convergence.md) bit for bit, and
`ConformanceTests` checks every row of the shared vectors in
[`trig-reference-v1`](../../../specs/fixtures/trig-reference-v1/README.md)
(62,154 rows, `sqrt`, `atan` and `atan2` included) by bit cast:

* range reduction to [-pi/4, pi/4]: the quotient is rounded to the nearest whole
  number with an add and a subtract of `MAGIC = 1.5 * 2^52`, then five pieces of
  pi/2 (four of at most 24 bits, so the products are exact) are subtracted
  (Cody-Waite); the quadrant `Int(k) & 3` picks `+-sin r` or `+-cos r`;
* ten-term Horner kernels on stored coefficients (`1/3!, 1/5!, ...`), no division;
* every constant is built with `Double(bitPattern:)`; every operation rounds once
  and nothing is fused (separate statements, no `addingProduct`);
* within **3 ulp** of the exact value on `|x| <= 2^29` (worst case 2.38 ulp);
* **`sin`, `cos` and `tan` return NaN** for NaN, the infinities and `|x| > 2^29`;
* `sin(-0)` is `-0`; `radians`/`degrees` are one multiply by a pre-divided
  constant; `atan2` of a NaN is NaN; `tan` keeps its pinned `+-1e308` pole
  sentinel (no accuracy claim is made for `tan`).

`Trig.sqrt` of a negative number still traps (`preconditionFailure`).
`Trig.sqrtChecked(_:)` is the non-trapping twin: it throws
`TrigError.negativeSquareRoot` for exactly those inputs, and the conformance test
asserts the vectors' `expect = error` rows against it.

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

A Swift implementation of trigonometric functions built entirely from scratch using mathematical series and Newton's method. No Foundation, no Darwin, no standard-library math functions — everything is computed from basic arithmetic.

## What It Does

This package computes:

| Function | Description | Algorithm |
|----------|-------------|-----------|
| `Trig.sin(_:)` | Sine (radians) | Cody-Waite reduction + 10-term Horner kernels (TRIG-REF-1) |
| `Trig.cos(_:)` | Cosine (radians) | Cody-Waite reduction + 10-term Horner kernels (TRIG-REF-1) |
| `Trig.tan(_:)` | Tangent (radians) | sin / cos |
| `Trig.sqrt(_:)` | Square root (traps on a negative) | Newton's (Babylonian) method |
| `Trig.sqrtChecked(_:)` | Square root, throws on a negative | same, non-trapping |
| `Trig.atan(_:)` | Arctangent | Taylor series + half-angle reduction |
| `Trig.atan2(_:_:)` | Four-quadrant arctangent | Quadrant logic + atan |
| `Trig.radians(_:)` | Degrees → radians | deg × RAD_PER_DEG (one multiply) |
| `Trig.degrees(_:)` | Radians → degrees | rad × DEG_PER_RAD (one multiply) |

Also exports `PI: Double = 3.141592653589793`.

## Where It Fits

This is a **PHY00 (physics layer 0)** leaf package — it has zero dependencies and is the foundation for all higher-level physics and geometry packages.

The same algorithms are implemented identically in Python, Go, TypeScript, Rust, Ruby, Elixir, Perl, and Lua.

## Usage

```swift
import Trig

let angle = Trig.radians(45.0)     // 0.7853981633974483
let s = Trig.sin(angle)            // 0.7071067811865476
let c = Trig.cos(angle)            // 0.7071067811865476
let t = Trig.tan(angle)            // 1.0
let r = Trig.sqrt(2.0)             // 1.4142135623730951
let a = Trig.atan(1.0)             // 0.7853981633974483 (π/4)
let a2 = Trig.atan2(1.0, 1.0)     // 0.7853981633974483 (π/4, first quadrant)
```

## Running Tests

```bash
swift test --enable-code-coverage --verbose
```

Or via the build tool:

```bash
./build-tool
```

## The Algorithms

### sin / cos — Cody-Waite reduction and Horner series

sin(x) = x − x³/3! + x⁵/5! − x⁷/7! + ...

x is split into whole quarter-turns and a small leftover, x = k·(π/2) + r with
|r| ≤ π/4, and the series is summed by Horner's rule on the stored coefficients
(ten terms, no division). See the literate comments in `Trig.swift` and
PHY00a sections 3.4 to 3.6.

### sqrt — Newton's Method

next_guess = (guess + x / guess) / 2

Quadratic convergence: correct digits double each iteration. Converges in ~4–15 iterations.

### atan — Taylor Series + Half-Angle Reduction

The Taylor series converges slowly near x = 1. The half-angle identity:

  atan(x) = 2 · atan( x / (1 + sqrt(1 + x²)) )

shrinks the argument to |y| ≤ 0.414, where 15 terms give full precision.

### atan2 — Quadrant Logic

Inspects the signs of both y and x to give the correct angle in all four quadrants, returning a value in (−π, π].

## Spec

See `code/specs/PHY00-trig.md` for the full specification.
