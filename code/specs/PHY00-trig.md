# PHY00: Trigonometry from First Principles

> **Amended by [PHY00a](PHY00a-trig-convergence.md) (TRIG-CONV).** The arithmetic of
> `sin`, `cos`, `tan`, `radians` and `degrees` is fixed to the bit by the reference
> algorithm TRIG-REF-1, and sections 2, 3.2 to 3.9, 5 and 6 below are amended to say
> so. **All seventeen `trig` ports have converged and merged** (PHY00a section 8,
> TC-1 to TC-17), so the amendments are unconditional: there is no longer an "older
> text" that a port may follow. The close-out (TC-18) is recorded in the change log at
> the end of this document.

## 1. Overview

The `trig` package is the foundational mathematical library for the Physics suite. Rather than delegating to opaque standard-library implementations, it computes trigonometric functions from first principles using Taylor (Maclaurin) series expansions. This teaches the actual algorithm that silicon uses under the hood — and provides a zero-dependency leaf package that every physics computation can build on.

## 2. Why Build Trig from Scratch?

When you call `Math.sin(x)` in any language, the runtime ultimately evaluates a polynomial approximation — the exact same Taylor series we implement here. By writing it ourselves we learn:

1. **How polynomials approximate transcendental functions** — the Maclaurin series converges because each successive term is smaller than the last.
2. **Why range reduction matters** — the series converges fastest near zero, so we fold any input into $[-\pi/4, \pi/4]$ first, keeping count of the quarter-turns removed.
3. **The precision/performance tradeoff** — more terms = more digits = more multiplications.

## 3. The Mathematics

### 3.1 The Constant $\pi$

$\pi$ is the ratio of a circle's circumference to its diameter. We store it as a pre-computed constant to 15+ decimal digits:

$$\pi = 3.141592653589793$$

This is sufficient for IEEE 754 double-precision floating point (~15-17 significant digits).

### 3.2 Sine — The Maclaurin Series

The sine function is defined by its infinite Taylor series centered at zero:

$$\sin(x) = x - \frac{x^3}{3!} + \frac{x^5}{5!} - \frac{x^7}{7!} + \frac{x^9}{9!} - \cdots = \sum_{n=0}^{\infty} \frac{(-1)^n \, x^{2n+1}}{(2n+1)!}$$

**How to read this:** Start with $x$ itself. Then subtract $x^3/6$. Then add $x^5/120$. Each term alternates sign and gets dramatically smaller (because the factorial in the denominator grows much faster than the power in the numerator). After about ten terms (on $|r| \le \pi/4$) the remaining terms are smaller than floating-point precision can represent, so TRIG-REF-1 stops at exactly ten (the $x^{19}$ term).

**Implementation strategy (TRIG-REF-1, PHY00a 3.5):** the coefficients $(-1)^n/(2n+1)!$ are stored once, each rounded once to a double and pinned by bit pattern, and the polynomial in $z = r^2$ is summed by Horner's rule, smallest term first, ten terms. There is no per-term division and no recurrence, so every port rounds identically. Cosine is the same shape, ten terms ($x^0$ to $x^{18}$), the leading $1 - z/2$ added last. (The older "20 terms" text described the pre-convergence ports: each built every term from the previous one, $\text{term}_{n+1} = \text{term}_n \times -x^2 / ((2n+2)(2n+3))$, with twenty or twenty-one terms; the grouping of that multiplication differed between ports and so did the last bits. PHY00a 3.5 has the kernels and the reasons for Horner.)

**The sign of zero.** $\sin(-0) = -0$ and $\sin(x) = x$ exactly for $0 < |x| < 2^{-27}$ (every subnormal included): the next term, $x^3/6$, is under half an ulp of $x$, so the correctly rounded answer *is* $x$. The early return is also the only thing that makes $\sin(-0) = -0$ true, because the general path computes $r + (r z) p$ and $-0 + (+0) = +0$ (PHY00a 3.3). $\cos$ returns exactly $1$ there.

### 3.3 Cosine — The Maclaurin Series

$$\cos(x) = 1 - \frac{x^2}{2!} + \frac{x^4}{4!} - \frac{x^6}{6!} + \cdots = \sum_{n=0}^{\infty} \frac{(-1)^n \, x^{2n}}{(2n)!}$$

Same idea as sine but starting at 1 instead of $x$, using even powers instead of odd.

### 3.4 Range Reduction

The Taylor series converges for all real $x$, but converges fastest when $|x|$ is small. Since sine and cosine are periodic with period $2\pi$, we can always reduce the input:

$$k = \text{round}(x / (\pi/2)), \qquad r = x - k \cdot \pi/2, \qquad |r| \le \pi/4$$

and the quadrant $k \bmod 4$ selects $\pm\sin r$ or $\pm\cos r$. The rounding is done with an add-and-subtract of $1.5 \cdot 2^{52}$, and the remainder $r$ is formed by **Cody-Waite subtraction** (PHY00a 3.4), not by the older `x - round(x / 2\pi) \cdot 2\pi`. The older form subtracts multiples of a single 53-bit $2\pi$, whose error ($\approx 2.4 \times 10^{-16}$ per period) is multiplied by the number of periods removed: about $10^{-11}$ off at $x = 10^6$ and arbitrary beyond $10^{19}$. Cody-Waite stores $\pi/2$ as a *sum* of pieces: four of at most 24 significant bits each, so that $k \cdot \text{piece}$ is exact for $|k| < 2^{29}$ (24 + 29 = 53 bits), and a fifth holding the remainder. Each subtraction removes one more slice of $k\pi/2$ from $x$, and $\pi/2$ is carried to about 149 bits. The quadrant is then `int(k) & 3`.

**Domain.** `sin`, `cos` and `tan` are defined for $|x| \le 2^{29}$ (about $5.4 \times 10^8$ radians). Outside it, for NaN and for the infinities, they return **NaN** (PHY00a 3.3 and 3.4.1). A finite value there would be wrong but look right: an earlier draft folded a huge argument down and returned $\sin(x/2^j)$, which no caller could tell from a sine; a NaN fails closed. The real remedy, Payne-Hanek reduction, is deferred (PHY00a 3.4.1). **On the BEAM** (Elixir), where a float cannot be NaN, "returns NaN" means **raises `ArithmeticError`**, as the lane already does for every non-finite result. The guard is written `not (|x| <= LIMIT)`, never `|x| > LIMIT`, so that one comparison catches NaN, both infinities and every finite value beyond the limit (PHY00a 3.6).

### 3.5 Degree/Radian Conversion

Radians are the natural unit for trigonometry (the Taylor series requires radians). Degrees are the human-friendly unit. The conversion is:

$$\text{radians} = \text{degrees} \times \frac{\pi}{180}$$
$$\text{degrees} = \text{radians} \times \frac{180}{\pi}$$

computed as **one multiply by a pre-divided constant** (`0x3F91DF46A2529D39` and `0x404CA5DC1A63C1F8`), not as `d * PI / 180`: one rounding, no intermediate overflow (PHY00a 3.7).

### 3.6 Tangent

Tangent is defined as the ratio of sine to cosine:

$$\tan(x) = \frac{\sin(x)}{\cos(x)}$$

**Geometric interpretation:** On the unit circle, if you draw a vertical line tangent to the circle at $(1, 0)$, then $\tan(x)$ is the $y$-coordinate where the ray at angle $x$ meets that line. This is the literal origin of the name "tangent."

**Undefined points (poles):** $\tan(x)$ is undefined wherever $\cos(x) = 0$, at $x = \pi/2 + k\pi$ for any integer $k$. At these points the function approaches $\pm\infty$. Implementations guard with a threshold ($|\cos(x)| < 10^{-15}$) and return the exact signed finite sentinel $\pm 10^{308}$, positive when $\sin(x) > 0$ and negative otherwise (PHY00a 3.8). The sentinel is a **pinned legacy contract, not an approximation**: the true tangent of the double nearest $\pi/2$ is about $1.6 \times 10^{16}$, and the shared corpus requires $10^{308}$ there. With the accurate reduction it fires only within about $10^{-15}$ of an odd multiple of $\pi/2$. The guard is false when $\cos$ is NaN, so $\tan(x)$ is NaN outside the domain of 3.4 (on the BEAM, it raises), and it makes $\cos = 0$ unreachable, so the division never traps. $\tan(\pm 0) = \pm 0$. No accuracy claim is made for `tan` (PHY00a 3.8).

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
  if x == 0: return x  # preserve signed zero
  if x == infinity: return infinity
  scaled = x
  result_scale = 1.0
  while scaled < 0.25:
    scaled = scaled * 4.0
    result_scale = result_scale * 0.5
  while scaled >= 4.0:
    scaled = scaled * 0.25
    result_scale = result_scale * 2.0
  guess = scaled if scaled >= 1.0 else 1.0
  repeat up to 60 times:
    next = (guess + scaled / guess) / 2.0
    if |next - guess| < 1e-15 * guess + 1e-300:
      return next * result_scale
    guess = next
  return guess * result_scale
```

The power-of-four normalization keeps Newton iteration in a compact range and
rescales the result by matching powers of two. This is required for accurate
small-normal and subnormal inputs under the fixed 60-iteration budget. The
convergence criterion `1e-15 * guess + 1e-300` then provides relative precision
within that normalized interval.

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

**Binary64 small-argument identity** (before either reduction): for
$|x| \leq 2^{-27}$, return `x` unchanged. The first omitted correction is
$|x|^3/3$, which is smaller than half a binary64 ulp throughout this interval,
so the correctly rounded result is exactly the input. Performing this check
before the half-angle division also preserves negative zero and prevents the
minimum positive or negative subnormal from underflowing to zero.

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
| $x$ or $y$ is NaN | NaN (checked **first**) |

**The NaN rule (added by PHY00a 3.9).** Written as a chain of comparisons, a NaN fails every test and falls through to the last row, which used to return a confident `0.0` (and in two ports `±π/2`) for an undefined question: the same defect as returning a finite sine beyond the domain. TRIG-REF-1 therefore tests for NaN before anything else, in every port.

**Pinned conventions that are not IEEE 754's.** These are contract, not accuracy (PHY00a 3.9): $\text{atan2}(-0, x<0) = +\pi$ (IEEE: $-\pi$); $\text{atan2}(\pm 0, \pm 0) = +0$; $\text{atan2}(\pm\infty, \pm\infty)$ is NaN because $y/x$ is; and a $y/x$ that overflows gives $\pm\pi/2$ through $\text{atan}(\pm\infty)$ (on the BEAM the division raises).

**Part II is pinned, not re-derived.** `sqrt` (3.7), `atan` (3.8) and `atan2` (3.9) keep the algorithms described above, which were already what the ports did; TRIG-REF-1 adds no new algorithm for them, it fixes their operation order (for instance the term update is evaluated left to right, $((\text{term} \cdot (-t^2)) \cdot (2n-1)) / (2n+1)$) and checks their bits against the shared vectors. PHY00a 3.9 has the pseudo-code and 3.9.1 the caveat: these functions are shown to agree across ports, **not** to be accurate beyond the $10^{-10}$ tolerance of section 5.

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

Dart exposes the `PI` contract constant as the idiomatic lower-camel `pi`,
alongside `twoPi` and `halfPi`. Its functions retain the contract's lowercase
names and `atan2(y, x)` argument order.

## 5. Precision Target

At ordinary finite points, implementations must agree with IEEE 754
double-precision standard-library results to within $1 \times 10^{-10}$ (10
decimal places) for inputs in the range $[-10^6, 10^6]$. A converged port is
far stronger than that: `sin` and `cos` are within **3 ulp of the exact value** on
$|x| \le 2^{29}$ (worst case measured: 2.38 ulp), `radians` and `degrees` within
1 ulp, and every result is **bit-identical** to the shared vectors of
[`fixtures/trig-reference-v1`](fixtures/trig-reference-v1/README.md) (PHY00a 4 and 6). Accuracy is claimed
only inside the domain of 3.4 and above $2^{-27}$ (below it the result is exact); beyond
$2^{29}$ the result is NaN (a raise on the BEAM), which is a contract case, not an error
bound. `tan` and the Part II functions (`sqrt`, `atan`, `atan2`) carry no ulp claim, only the bit-agreement of PHY00a 3.9.1.
The documented tangent pole sentinel, the `atan2` NaN rule and branch-cut convention, and
$\sin(-0) = -0$ are explicit contract cases, not standard-library-agreement points.

### 5.1 Language-neutral conformance corpus

The versioned machine-readable oracle lives at
`fixtures/phy00-phy01-v1/cases/trig.json` and is validated by the closed Draft
2020-12 `fixtures/phy00-phy01-v1/schema.json`. It is normative for shared case
identity, binary64 input values, expected outcomes, and tolerances. Package-
native tests may add coverage but may not weaken or reinterpret a shared case.

The corpus represents finite values as tagged decimal strings and represents
NaN and infinities as tagged symbols. This preserves minimum subnormal input,
maximum finite input, and the sign of zero across JSON implementations. Exact
comparison of NaN means `isNaN`; exact zero comparison includes the zero sign.
The included `atan2(-0, -1)` case returns positive $\pi$, consistent with the
contract range $(-\pi, \pi]$. The `atan` boundary cases require exact identity
for negative zero, a representative tiny finite value, and both signed minimum
subnormals.

## 6. Cross-Language Parity

The package is implemented in all **seventeen** `trig` lanes: the fifteen established
ones (**C#, Dart, Elixir, F#, Go, Haskell, Java, Kotlin, Lua, Perl, Python, Ruby, Rust,
Swift, and TypeScript**) and the emerging **C** and **C++** lanes (which do not enter
the established-lane denominator). Every implementation uses the same first-principles
algorithm, PHY00a's TRIG-REF-1: Cody-Waite range reduction to $[-\pi/4, \pi/4]$ and ten-term
Horner kernels, with the `tan`, `atan2` and Part II rules of 3.6 to 3.9. Each
port is bit-identical to the shared vectors of `fixtures/trig-reference-v1` (PHY00a 6)
and also consumes the versioned `phy00-phy01-v1` cases. (Per-port notes, such as
Elixir's `ArithmeticError` for non-finite results and Dart's VM-only run, are in
PHY00a 8.) The v1 corpus carries constants, representative
finite points, tangent poles, quadrant and signed-zero `atan2`, conversions,
and square-root boundaries. Together with package-native extensions, the full
parity suites validate:

1. Known exact values: $\sin(0) = 0$, $\cos(0) = 1$, $\sin(\pi/2) = 1$, $\cos(\pi) = -1$
2. Symmetry: $\sin(-x) = -\sin(x)$, $\cos(-x) = \cos(x)$, including $\sin(-0) = -0$
3. Pythagorean identity: $\sin^2(x) + \cos^2(x) = 1$ for arbitrary $x$
4. Large inputs: range reduction handles $x = 1000\pi$ correctly; $|x| > 2^{29}$, NaN and the infinities give NaN (a raise on the BEAM)
5. Degree conversion round-trips: `degrees(radians(45.0)) == 45.0`
6. `sqrt(4) == 2`, `sqrt(2) * sqrt(2) ≈ 2.0`
7. `tan(π/4) ≈ 1.0`, `tan(-π/4) ≈ -1.0`
8. `atan(1) ≈ π/4`, `atan(-1) ≈ -π/4`, `atan(√3) ≈ π/3`, and tiny inputs preserve their exact binary64 value and zero sign
9. `atan2(0, 1) == 0`, `atan2(1, 0) == π/2`, `atan2(0, -1) == π`, `atan2(-1, 0) == -π/2`
10. All four atan2 quadrant cases (Q1–Q4), and `atan2` of a NaN is NaN

Note: Perl uses `atan2_approx(y, x)` with positional args (not keyword); Lua uses `trig.atan2(y, x)`. Swift uses `Trig.atan2(_:_:)` with the standard `(y, x)` argument order matching POSIX `atan2`.

## 7. Change Log

| Date | Change |
|------|--------|
| (original) | Taylor-series `sin`/`cos`, range reduction by `round(x / 2π)`, twenty terms, `tan`, `sqrt`, `atan`, `atan2`. |
| 2026-11-16, TC-1 and TC-2 | Banner added; 2, 3.2 to 3.5, 5 and 6 amended: Horner kernels on pinned coefficients, Cody-Waite reduction, domain `|x| <= 2^29` with NaN beyond, pre-divided degree constants, bit-identity to `trig-reference-v1`. Python, Rust and TypeScript converged. |
| 2026-11-16, TC-18 | Close-out. All seventeen ports merged, so the amendments are unconditional. **Changed:** the banner; 3.2 (ten terms, the cosine kernel, `sin(-0) = -0` and the `2^-27` early return); 3.4 (why Cody-Waite replaces `round(x / 2π)`, the exact bit budget, the domain with the BEAM raise); 3.6 (the `tan` pole sentinel as a pinned legacy contract, NaN and domain behaviour); 3.9 (the `atan2` NaN rule, the pinned non-IEEE conventions, Part II pinned not re-derived); 5 (where accuracy is and is not claimed); 6 (seventeen ports, not fifteen "being reconciled"; added checks). **Why:** repo rule that specs stay in sync with merged code, and PHY00a OQ-9 (amend at TC-1 for the algorithm, again at TC-18 for the close-out). PHY00a owns the details; this document keeps the ideas and cites it by section. No algorithm changed in this commit. |
