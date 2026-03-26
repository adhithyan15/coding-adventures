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

## 4. API Surface

All functions are module-level (not attached to an object). The package exposes:

| Symbol | Type | Description |
|--------|------|-------------|
| `PI` | constant | $\pi$ to double-precision accuracy |
| `sin(x)` | function | Sine of $x$ (radians) via Taylor series |
| `cos(x)` | function | Cosine of $x$ (radians) via Taylor series |
| `radians(deg)` | function | Convert degrees to radians |
| `degrees(rad)` | function | Convert radians to degrees |

## 5. Precision Target

All implementations must agree with IEEE 754 double-precision standard-library results to within $1 \times 10^{-10}$ (10 decimal places) for any input in the range $[-10^6, 10^6]$.

## 6. Cross-Language Parity

The package is implemented identically across all 6 host languages (Python, Go, Ruby, TypeScript, Rust, Elixir). Each implementation uses the same algorithm (iterative term computation, range reduction) and passes the same test cases validating:

1. Known exact values: $\sin(0) = 0$, $\cos(0) = 1$, $\sin(\pi/2) = 1$, $\cos(\pi) = -1$
2. Symmetry: $\sin(-x) = -\sin(x)$, $\cos(-x) = \cos(x)$
3. Pythagorean identity: $\sin^2(x) + \cos^2(x) = 1$ for arbitrary $x$
4. Large inputs: range reduction handles $x = 1000\pi$ correctly
5. Degree conversion round-trips: `degrees(radians(45.0)) == 45.0`
