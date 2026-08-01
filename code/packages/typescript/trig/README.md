# trig

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
