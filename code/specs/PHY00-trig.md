# PHY00: Trigonometry from First Principles

## 1. Overview

The `trig` package is the foundational mathematical library for the Physics suite. Rather than delegating to opaque standard-library implementations, it computes trigonometric functions from first principles using Taylor (Maclaurin) series expansions. This teaches the actual algorithm that silicon uses under the hood — and provides a zero-dependency leaf package that every physics computation can build on.

## 2. Why Build Trig from Scratch?

When you call `Math.sin(x)` in any language, the runtime ultimately evaluates a polynomial approximation — the exact same Taylor series we implement here. By writing it ourselves we learn:

1. **How polynomials approximate transcendental functions** — the Maclaurin series converges because each successive term is smaller than the last.
2. **Why range reduction matters** — the series converges fastest near zero, so we fold any input into $[-\pi, \pi]$ first.
3. **The precision/performance tradeoff** — more terms = more digits = more multiplications.

## 3. The Mathematics

### 3.1 The Constant $\pi$

$\pi$ is the ratio of a circle's circumference to its diameter. We store it as a pre-computed constant to 15+ decimal digits:

$$\pi = 3.141592653589793$$

This is sufficient for IEEE 754 double-precision floating point (~15-17 significant digits).

### 3.2 Sine — The Maclaurin Series

The sine function is defined by its infinite Taylor series centered at zero:

$$\sin(x) = x - \frac{x^3}{3!} + \frac{x^5}{5!} - \frac{x^7}{7!} + \frac{x^9}{9!} - \cdots = \sum_{n=0}^{\infty} \frac{(-1)^n \, x^{2n+1}}{(2n+1)!}$$

**How to read this:** Start with $x$ itself. Then subtract $x^3/6$. Then add $x^5/120$. Each term alternates sign and gets dramatically smaller (because the factorial in the denominator grows much faster than the power in the numerator). After about 10-12 terms, the remaining terms are smaller than floating-point precision can represent.

**Implementation strategy:** Rather than computing $x^n$ and $n!$ separately (which overflow quickly), we compute each term from the previous one:

$$\text{term}_{n+1} = \text{term}_n \times \frac{-x^2}{(2n+2)(2n+3)}$$

This is numerically stable and avoids overflow entirely.

### 3.3 Cosine — The Maclaurin Series

$$\cos(x) = 1 - \frac{x^2}{2!} + \frac{x^4}{4!} - \frac{x^6}{6!} + \cdots = \sum_{n=0}^{\infty} \frac{(-1)^n \, x^{2n}}{(2n)!}$$

Same idea as sine but starting at 1 instead of $x$, using even powers instead of odd.

### 3.4 Range Reduction

The Taylor series converges for all real $x$, but converges fastest when $|x|$ is small. Since sine and cosine are periodic with period $2\pi$, we can always reduce the input:

$$x_{\text{reduced}} = x - 2\pi \times \text{round}(x / 2\pi)$$

This maps any input to the range $[-\pi, \pi]$ where the series converges within 12-15 terms.

### 3.5 Degree/Radian Conversion

Radians are the natural unit for trigonometry (the Taylor series requires radians). Degrees are the human-friendly unit. The conversion is:

$$\text{radians} = \text{degrees} \times \frac{\pi}{180}$$
$$\text{degrees} = \text{radians} \times \frac{180}{\pi}$$

### 3.6 Tangent

Tangent is defined as the ratio of sine to cosine:

$$\tan(x) = \frac{\sin(x)}{\cos(x)}$$

**Geometric interpretation:** On the unit circle, if you draw a vertical line tangent to the circle at $(1, 0)$, then $\tan(x)$ is the $y$-coordinate where the ray at angle $x$ meets that line. This is the literal origin of the name "tangent."

**Undefined points (poles):** $\tan(x)$ is undefined wherever $\cos(x) = 0$, at $x = \pi/2 + k\pi$ for any integer $k$. At these points the function approaches $\pm\infty$. Implementations guard with a threshold ($|\cos(x)| < 10^{-15}$) and return the largest representable float.

**Worked example:**

$$\tan(\pi/4) = \frac{\sin(\pi/4)}{\cos(\pi/4)} = \frac{\sqrt{2}/2}{\sqrt{2}/2} = 1.0$$

We use our own `sin` and `cos` — no standard library `tan` function.

### 3.7 Square Root

Square root is computed via **Newton's (Babylonian) method**, one of the oldest numerical algorithms (Babylonian clay tablets, ~1700 BCE). The recurrence:

$$\text{next} = \frac{\text{guess} + x / \text{guess}}{2}$$

**Why it works:** If $\text{guess} < \sqrt{x}$, then $x/\text{guess} > \sqrt{x}$. Their average is closer from both sides. The method has **quadratic convergence**: the number of correct digits doubles each iteration.

**Convergence table for $\sqrt{2}$:**

| Iteration | Guess | Correct digits |
|-----------|-------|----------------|
| 0 | 2.000000 | 0 |
| 1 | 1.500000 | 1 |
| 2 | 1.416667 | 2 |
| 3 | 1.414216 | 5 |
| 4 | 1.41421356237... | 11+ |

**Implementation:**

```
function sqrt(x):
  if x < 0: raise error
  if x == 0: return 0.0
  guess = x if x >= 1.0 else 1.0
  repeat up to 60 times:
    next = (guess + x / guess) / 2.0
    if |next - guess| < 1e-15 * guess + 1e-300: return next
    guess = next
  return guess
```

The convergence criterion `1e-15 * guess + 1e-300` handles both relative precision (for large values) and subnormal inputs safely.

`sqrt` is also used internally by `atan_core` for the half-angle reduction — which is why it must be implemented from scratch and not delegated to any standard library.

### 3.8 Arctangent

`atan(x)` is the inverse of tangent: given a ratio, return the angle in $(-\pi/2, \pi/2)$.

**The Taylor series** for atan:

$$\text{atan}(x) = x - \frac{x^3}{3} + \frac{x^5}{5} - \frac{x^7}{7} + \cdots \quad \text{for } |x| \leq 1$$

This converges for $|x| \leq 1$, but slowly near $x = 1$ (requires ~50 terms for full precision).

**Layer 1 — Outer range reduction** (for $|x| > 1$):

$$\text{atan}(x) = \frac{\pi}{2} - \text{atan}\!\left(\frac{1}{x}\right) \quad x > 1$$
$$\text{atan}(x) = -\frac{\pi}{2} - \text{atan}\!\left(\frac{1}{x}\right) \quad x < -1$$

*Proof:* If $\theta = \text{atan}(x)$, then $\tan(\pi/2 - \theta) = \cot(\theta) = 1/x$, so $\text{atan}(1/x) = \pi/2 - \theta$.

**Layer 2 — Half-angle reduction** (inside `atan_core`):

$$\text{atan}(x) = 2 \cdot \text{atan}\!\left(\frac{x}{1 + \sqrt{1 + x^2}}\right)$$

After one application, $|x| \leq 1$ shrinks to $|y| \leq \tan(\pi/8) \approx 0.414$, where the Taylor series converges in ~15 terms. We use our own `sqrt` here.

**Iterative term computation:**

$$\text{term}_0 = t, \quad \text{term}_n = \text{term}_{n-1} \times (-t^2) \times \frac{2n-1}{2n+1}$$

The final result is multiplied by $2$ to undo the half-angle halving.

### 3.9 Two-Argument Arctangent (atan2)

`atan2(y, x)` returns the angle in $(-\pi, \pi]$ that the vector $(x, y)$ makes with the positive $x$-axis. Unlike `atan(y/x)`, it correctly handles all four quadrants.

**Why `atan(y/x)` is insufficient:** If $y = -1, x = -1$, then $y/x = 1$ and $\text{atan}(1) = \pi/4$. But the actual point $(-1, -1)$ is in the third quadrant — the angle should be $-3\pi/4$.

**Quadrant diagram:**

```
         y > 0
     Q2  |  Q1       atan2 ∈ (π/2,  π]  for Q2
   ------+------  x  atan2 ∈ (0,    π/2) for Q1
     Q3  |  Q4       atan2 ∈ (-π,  -π/2) for Q3
         y < 0       atan2 ∈ (-π/2,  0)  for Q4
```

**Decision table:**

| Condition | atan2(y, x) |
|-----------|-------------|
| $x > 0$ | $\text{atan}(y/x)$ |
| $x < 0, y \geq 0$ | $\text{atan}(y/x) + \pi$ |
| $x < 0, y < 0$ | $\text{atan}(y/x) - \pi$ |
| $x = 0, y > 0$ | $\pi/2$ |
| $x = 0, y < 0$ | $-\pi/2$ |
| $x = 0, y = 0$ | $0$ (undefined, by convention) |

## 4. API Surface

All functions are module-level (not attached to an object). The package exposes:

| Symbol | Type | Description |
|--------|------|-------------|
| `PI` | constant | $\pi$ to double-precision accuracy |
| `sin(x)` | function | Sine of $x$ (radians) via Taylor series |
| `cos(x)` | function | Cosine of $x$ (radians) via Taylor series |
| `tan(x)` | function | Tangent of $x$ (radians) — sin/cos ratio |
| `sqrt(x)` | function | Square root via Newton's method |
| `atan(x)` | function | Arctangent of $x$, result in $(-\pi/2, \pi/2)$ |
| `atan2(y, x)` | function | Four-quadrant arctangent, result in $(-\pi, \pi]$ |
| `radians(deg)` | function | Convert degrees to radians |
| `degrees(rad)` | function | Convert radians to degrees |

### Language-specific naming

| Function | Go | Perl | Lua |
|----------|-----|------|-----|
| `sin` | `Sin` | `sin_approx` | `trig.sin` |
| `cos` | `Cos` | `cos_approx` | `trig.cos` |
| `tan` | `Tan` | `tan_approx` | `trig.tan` |
| `sqrt` | `Sqrt` | `sqrt_approx` | `trig.sqrt` |
| `atan` | `Atan` | `atan_approx` | `trig.atan` |
| `atan2` | `Atan2` | `atan2_approx` | `trig.atan2` |

## 5. Precision Target

All implementations must agree with IEEE 754 double-precision standard-library results to within $1 \times 10^{-10}$ (10 decimal places) for any input in the range $[-10^6, 10^6]$.

## 6. Cross-Language Parity

The package is implemented identically across all 9 host languages: **Python, Go, TypeScript, Rust, Ruby, Elixir, Perl, Lua, and Swift**. Each implementation uses the same algorithm (iterative term computation, range reduction) and passes the same test cases validating:

1. Known exact values: $\sin(0) = 0$, $\cos(0) = 1$, $\sin(\pi/2) = 1$, $\cos(\pi) = -1$
2. Symmetry: $\sin(-x) = -\sin(x)$, $\cos(-x) = \cos(x)$
3. Pythagorean identity: $\sin^2(x) + \cos^2(x) = 1$ for arbitrary $x$
4. Large inputs: range reduction handles $x = 1000\pi$ correctly
5. Degree conversion round-trips: `degrees(radians(45.0)) == 45.0`
6. `sqrt(4) == 2`, `sqrt(2) * sqrt(2) ≈ 2.0`
7. `tan(π/4) ≈ 1.0`, `tan(-π/4) ≈ -1.0`
8. `atan(1) ≈ π/4`, `atan(-1) ≈ -π/4`, `atan(√3) ≈ π/3`
9. `atan2(0, 1) == 0`, `atan2(1, 0) == π/2`, `atan2(0, -1) == π`, `atan2(-1, 0) == -π/2`
10. All four atan2 quadrant cases (Q1–Q4)

Note: Perl uses `atan2_approx(y, x)` with positional args (not keyword); Lua uses `trig.atan2(y, x)`. Swift uses `Trig.atan2(_:_:)` with the standard `(y, x)` argument order matching POSIX `atan2`.
