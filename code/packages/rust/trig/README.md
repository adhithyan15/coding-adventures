# trig

Trigonometric functions computed from first principles using Maclaurin (Taylor) series. No standard-library trig functions are used — every value is built up from basic arithmetic.

## Layer

PHY00 — a leaf package with zero dependencies.

## What it does

This crate provides `sin`, `cos`, and angle-conversion utilities. Instead of delegating to `libm` or hardware instructions, it evaluates the infinite Maclaurin series (truncated to 20 terms) directly:

```
sin(x) = x - x³/3! + x⁵/5! - x⁷/7! + ...
cos(x) = 1 - x²/2! + x⁴/4! - x⁶/6! + ...
```

A range-reduction step maps any input to [-pi, pi] before summing, ensuring fast convergence and minimal floating-point error.

## API

| Function       | Description                              |
|----------------|------------------------------------------|
| `sin(x)`       | Sine of `x` (radians)                   |
| `cos(x)`       | Cosine of `x` (radians)                 |
| `radians(deg)` | Convert degrees to radians               |
| `degrees(rad)` | Convert radians to degrees               |
| `PI`           | The constant pi (3.141592653589793)      |

## Usage

```rust
use trig::{sin, cos, radians, PI};

let angle = radians(45.0);
let s = sin(angle);
let c = cos(angle);
assert!((s * s + c * c - 1.0).abs() < 1e-10);
```

## Building and testing

```sh
cargo build
cargo test
```
