# trig — Java

Trigonometric functions built from first principles using Maclaurin series.
No `Math.sin`, `Math.cos`, or any other standard library math functions are
used internally — every result is derived from addition, multiplication, and
division alone.

## What It Implements

| Function | Description |
|----------|-------------|
| `Trig.sin(x)` | Sine (radians), 20-term Maclaurin series |
| `Trig.cos(x)` | Cosine (radians), 20-term Maclaurin series |
| `Trig.tan(x)` | Tangent = sin/cos, with pole guard |
| `Trig.sqrt(x)` | Square root via Newton's (Babylonian) method |
| `Trig.atan(x)` | Arctangent, Taylor series with half-angle reduction |
| `Trig.atan2(y, x)` | Four-quadrant arctangent |
| `Trig.radians(deg)` | Degrees → radians |
| `Trig.degrees(rad)` | Radians → degrees |
| `Trig.PI` | π constant to full double precision |

## How It Works

### Maclaurin Series

Both `sin` and `cos` use the standard Maclaurin expansion:

```
sin(x) = x - x³/3! + x⁵/5! - x⁷/7! + ...
cos(x) = 1 - x²/2! + x⁴/4! - x⁶/6! + ...
```

Rather than computing each term from scratch (which requires large factorials),
each term is derived from the previous one by multiplying by a simple fraction:

```
sin: term_k = term_{k-1} * (-x²) / (2k)(2k+1)
cos: term_k = term_{k-1} * (-x²) / (2k-1)(2k)
```

### Range Reduction

For large inputs the series is slow to converge. Since sin and cos are periodic
with period 2π, we first reduce `x` into `[-π, π]` using `x % (2π)`.

### Square Root — Newton's Method

```
guess_next = (guess + x / guess) / 2
```

Quadratic convergence: the number of correct digits doubles each iteration.
Full double precision typically in ≤ 15 iterations.

## Usage

```java
import com.codingadventures.trig.Trig;

double s = Trig.sin(Trig.PI / 4);          // ≈ 0.7071...
double c = Trig.cos(Trig.radians(60.0));   // ≈ 0.5
double r = Trig.sqrt(2.0);                  // ≈ 1.41421...
double a = Trig.atan2(1.0, 1.0);           // ≈ π/4
```

## Running Tests

```bash
gradle test
```

## Part of the Coding Adventures series

This package is the Java counterpart to the Python, Rust, Go, and TypeScript
implementations of `trig`. All use the same algorithm so cross-language output
is identical to full double precision.
