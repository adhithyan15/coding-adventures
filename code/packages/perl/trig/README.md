# trig

Trigonometric functions implemented from first principles using Taylor (Maclaurin) series. No math library is used — everything is built from basic arithmetic.

## Layer

**PHY00** — this is a leaf package with no dependencies. It is used by the `wave` package.

## What it does

This package implements `sin_taylor` and `cos_taylor` by summing terms of their Maclaurin series expansions. Range reduction normalizes inputs to [-pi, pi] before evaluation, ensuring accuracy even for very large angles.

## API

| Function            | Description                                      |
|---------------------|--------------------------------------------------|
| `PI`                | Pi constant to double-precision accuracy         |
| `sin_taylor($x)`    | Sine of x (radians) via Maclaurin series         |
| `cos_taylor($x)`    | Cosine of x (radians) via Maclaurin series       |
| `radians($deg)`     | Convert degrees to radians                       |
| `degrees($rad)`     | Convert radians to degrees                       |

## Usage

```perl
use CodingAdventures::Trig qw(sin_taylor cos_taylor radians degrees PI);

say sin_taylor(PI / 2);      # 1.0
say cos_taylor(0);            # 1.0
say sin_taylor(radians(30));  # 0.5
say degrees(PI);              # 180.0
```

## How it works

The Maclaurin series for sine is:

    sin(x) = x - x^3/3! + x^5/5! - x^7/7! + ...

Each term is computed iteratively from the previous term to avoid large intermediate factorials:

    term_{n+1} = term_n * (-x^2) / ((2n+2)(2n+3))

Cosine follows the same pattern with even powers. Twenty terms are computed, yielding full double-precision accuracy for any input.
