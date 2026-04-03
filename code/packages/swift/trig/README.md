# trig — Trigonometric Functions from First Principles (Swift)

A Swift implementation of trigonometric functions built entirely from scratch using mathematical series and Newton's method. No Foundation, no Darwin, no standard-library math functions — everything is computed from basic arithmetic.

## What It Does

This package computes:

| Function | Description | Algorithm |
|----------|-------------|-----------|
| `Trig.sin(_:)` | Sine (radians) | 20-term Maclaurin series |
| `Trig.cos(_:)` | Cosine (radians) | 20-term Maclaurin series |
| `Trig.tan(_:)` | Tangent (radians) | sin / cos |
| `Trig.sqrt(_:)` | Square root | Newton's (Babylonian) method |
| `Trig.atan(_:)` | Arctangent | Taylor series + half-angle reduction |
| `Trig.atan2(_:_:)` | Four-quadrant arctangent | Quadrant logic + atan |
| `Trig.radians(_:)` | Degrees → radians | deg × π/180 |
| `Trig.degrees(_:)` | Radians → degrees | rad × 180/π |

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

### sin / cos — Maclaurin Series

sin(x) = x − x³/3! + x⁵/5! − x⁷/7! + ...

Each term is computed from the previous one to avoid large factorials. Range reduction to [−π, π] happens first.

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
