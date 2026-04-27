# trig — Kotlin

Trigonometric functions built from first principles using Maclaurin series.
No `kotlin.math.sin`, `kotlin.math.cos`, or any other standard library math
functions are used internally — every result is derived from addition,
multiplication, and division alone.

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

## Usage

```kotlin
import com.codingadventures.trig.Trig

val s = Trig.sin(Trig.PI / 4)           // ≈ 0.7071...
val c = Trig.cos(Trig.radians(60.0))    // ≈ 0.5
val r = Trig.sqrt(2.0)                   // ≈ 1.41421...
val a = Trig.atan2(1.0, 1.0)            // ≈ π/4
```

## Running Tests

```bash
gradle test
```

## Part of the Coding Adventures series

Kotlin counterpart to the Python, Rust, Go, TypeScript, and Java implementations.
All use the same algorithm for identical results across languages.
