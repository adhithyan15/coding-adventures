# trig

## Converged on TRIG-REF-1 (0.3.0)

`sin`, `cos`, `radians` and `degrees` now follow the reference algorithm of
[PHY00a](../../../specs/PHY00a-trig-convergence.md), bit for bit, and are checked against
the shared vectors in
[`trig-reference-v1`](../../../specs/fixtures/trig-reference-v1/README.md):

* range reduction to [-pi/4, pi/4] by the add-and-subtract-`MAGIC` rounding and five
  exact-product pieces of pi/2 (Cody-Waite), with the quadrant `k mod 4` picking
  `+-sin r` or `+-cos r`;
* ten-term Horner kernels on stored coefficients (`1/3!, 1/5!, ...`), no division;
* every constant is a bit pattern; every operation rounds once and nothing is fused;
* within **3 ulp** of the exact value on `|x| <= 2**29` (worst case 2.38 ulp);
* **`sin`, `cos` and `tan` return NaN** for NaN, the infinities and `|x| > 2**29`;
* `sin(-0)` is `-0`, `radians`/`degrees` are one multiply by a pre-divided constant,
  `atan2` of a NaN is NaN, and `tan` keeps its pinned `+-1e308` pole sentinel.

(The older 20-term recurrence text below describes the algorithm before this version.)

## Exact small arctangent

`atan` returns `x` unchanged when `|x| <= 2^-27`, before any half-angle
reduction. This exact binary64 identity preserves negative zero and both signs
of the minimum subnormal; the shared PHY00 fixture locks those boundaries.

## Full-range square root

The square-root API follows PHY00 across the full binary64 range. It preserves
negative zero, returns positive infinity and NaN unchanged, rejects negative
inputs with the lane-native error, normalizes by powers of four into `[0.25, 4)`,
and then applies at most 60 Newton steps without calling the host square-root
routine. Boundary behavior is checked against
[`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json).

Trigonometric functions computed from first principles using Taylor (Maclaurin) series. No `Math.sin` or `Math.cos` — just arithmetic.

## Layer

PHY00 — foundational mathematics, no dependencies.

## What it does

This package implements sine and cosine by summing the terms of their infinite series representations. Range reduction (modulo 2pi) keeps inputs small for fast convergence, and 20 terms of the series give full double-precision accuracy (~15 significant digits).

## API

| Export         | Description                                      |
|----------------|--------------------------------------------------|
| `PI`           | The constant pi to double-precision accuracy     |
| `sin(x)`       | Sine of x (radians) via Maclaurin series         |
| `cos(x)`       | Cosine of x (radians) via Maclaurin series       |
| `radians(deg)` | Convert degrees to radians                       |
| `degrees(rad)` | Convert radians to degrees                       |

## Usage

```typescript
import { sin, cos, radians, PI } from "trig";

sin(PI / 6);       // 0.5
cos(radians(60));   // 0.5
```

## How it works

The Maclaurin series for sine:

```
sin(x) = x - x^3/3! + x^5/5! - x^7/7! + ...
```

Each term is derived from the previous one iteratively (no factorial or power overflow). Range reduction maps any input to [-pi, pi] before summation.

## Building

```bash
npm install
npx jest --verbose
```
