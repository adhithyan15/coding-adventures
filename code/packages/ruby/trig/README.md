# Trig

Trigonometric functions implemented from first principles using Maclaurin (Taylor) series. No standard library math functions are used — everything is built from basic arithmetic.

**Layer:** PHY00 (leaf package, no dependencies)

## What It Does

This package provides sine, cosine, and degree/radian conversion, all computed from scratch using infinite series approximations truncated to 20 terms.

## How It Works

### Maclaurin Series

The Maclaurin series expresses functions as infinite sums of polynomial terms:

```
sin(x) = x - x^3/3! + x^5/5! - x^7/7! + ...
cos(x) = 1 - x^2/2! + x^4/4! - x^6/6! + ...
```

Each term is computed iteratively from the previous one (avoiding large factorials):

```
sin: term_n = term_{n-1} * (-x^2) / (2n * (2n+1))
cos: term_n = term_{n-1} * (-x^2) / ((2n-1) * 2n)
```

### Range Reduction

Before computing, angles are reduced to [-PI, PI] using the periodicity of sine and cosine. This ensures fast convergence with just 20 terms.

## API

| Method | Description |
|---|---|
| `Trig.sin(x)` | Sine of x (radians) |
| `Trig.cos(x)` | Cosine of x (radians) |
| `Trig.radians(deg)` | Convert degrees to radians |
| `Trig.degrees(rad)` | Convert radians to degrees |
| `Trig::PI` | Pi to double precision |
| `Trig::TWO_PI` | 2 * Pi |

## Usage

```ruby
require_relative 'lib/trig'

Trig.sin(Trig::PI / 2)   # => 1.0
Trig.cos(0)               # => 1.0
Trig.radians(180)         # => 3.141592653589793
Trig.degrees(Trig::PI)    # => 180.0
```

## Running Tests

```bash
ruby test/test_trig.rb
```
