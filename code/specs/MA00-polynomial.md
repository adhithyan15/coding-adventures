# MA00 — Polynomial: Coefficient-Array Polynomial Arithmetic

## Overview

A **polynomial** is a mathematical expression built from a variable *x* and a list
of constant **coefficients**. The polynomial `3 + 0x + 2x²` has three terms.

We represent a polynomial as a **coefficient array** where the **index equals the
degree** of the term:

```
[3, 0, 2]
 ^  ^  ^
 |  |  └── coefficient of x²  (degree 2)
 |  └───── coefficient of x¹  (degree 1)
 └──────── coefficient of x⁰  (degree 0, the constant)
```

This "index = degree" convention is sometimes called **little-endian** because
the lowest-degree term comes first. It makes addition trivially position-aligned
and keeps Horner's method (fast evaluation) natural to read.

### Why This Package Exists

Polynomial arithmetic is the foundation of three important layers in the
coding-adventures stack:

1. **GF(2^8) arithmetic (MA01)** — The Galois Field used by Reed-Solomon and AES
   is defined by arithmetic in a polynomial ring modulo an irreducible polynomial.
   Every element of GF(256) *is* a polynomial over GF(2).

2. **Reed-Solomon error correction (MA02)** — A Reed-Solomon codeword is a
   polynomial evaluated at specific points. Encoding is polynomial multiplication.
   Decoding uses the Euclidean algorithm for polynomial GCD.

3. **Checksums and CRCs** — A CRC is the remainder after polynomial division,
   modulo a generator polynomial over GF(2).

---

## How It Works

### The Zero Polynomial

The zero polynomial has *no* non-zero terms. We represent it as the empty array
`[]`. It is the additive identity: `p + zero = p` for any polynomial `p`.

All operations that produce a result normalize their output — trailing zeros
(high-degree coefficients that are zero) are stripped. So `[1, 0, 0]` and `[1]`
both represent the constant polynomial `1`.

```
normalize([1, 0, 0]) → [1]
normalize([0])       → []
normalize([])        → []
```

### degree

The degree of a polynomial is the highest exponent with a non-zero coefficient.
By convention:
- `degree([3, 0, 2]) = 2`   (highest non-zero: index 2)
- `degree([7]) = 0`          (constant polynomial, degree zero)
- `degree([]) = -1`          (zero polynomial, degree minus-one by convention)

The `-1` for the zero polynomial is important: it lets the polynomial long division
loop terminate cleanly (`degree(remainder) < degree(divisor)`).

### add

Addition is **term-by-term**: add matching coefficients, padding the shorter
polynomial with zeros.

```
  [1, 2, 3]      = 1 + 2x + 3x²
+ [4, 5]         = 4 + 5x
──────────────
  [5, 7, 3]      = 5 + 7x + 3x²
```

Step through: `1+4=5`, `2+5=7`, `3+0=3`. The degree-2 term had no partner so it
carried through unchanged.

### subtract

Subtraction is the same as addition but with the second polynomial negated.

```
  [5, 7, 3]      = 5 + 7x + 3x²
- [1, 2, 3]      = 1 + 2x + 3x²
──────────────
  [4, 5, 0]  →  normalize →  [4, 5]
```

Note that `3x² - 3x² = 0`, and normalize strips the trailing zero.

### multiply

Multiplication is **convolution**. Each term in `a` multiplies each term in `b`.
If `a` has degree `m` and `b` has degree `n`, the result has degree `m + n`.

```
  a = [1, 2]     = 1 + 2x
× b = [3, 4]     = 3 + 4x
────────────────────────────
result = new array of length m+n+1, all zeros

For each i in a, for each j in b:
  result[i+j] += a[i] * b[j]

  i=0, j=0: result[0] += 1*3 = 3
  i=0, j=1: result[1] += 1*4 = 4
  i=1, j=0: result[1] += 2*3 = 6    → result[1] = 10
  i=1, j=1: result[2] += 2*4 = 8

result = [3, 10, 8]         = 3 + 10x + 8x²

Verify: (1 + 2x)(3 + 4x) = 3 + 4x + 6x + 8x² = 3 + 10x + 8x²  ✓
```

### divmod — Polynomial Long Division

Polynomial long division is the same algorithm you learned in school, adapted for
polynomials. Given `a / b`, we find quotient `q` and remainder `r` such that:

```
a = b × q + r
degree(r) < degree(b)
```

**Step-by-step example**: Divide `2x³ + 3x² + x + 5` by `x + 2`.

Represent as arrays:
```
a = [5, 1, 3, 2]    (5 + x + 3x² + 2x³)
b = [2, 1]          (2 + x)
```

The algorithm keeps a working copy of the remainder, starting as `a`:

```
Step 1: remainder = [5, 1, 3, 2], degree = 3
        b has degree 1.  3 >= 1, so we can divide.
        leading term of remainder:  2x³  (coeff 2 at index 3)
        leading term of divisor:    1x¹  (coeff 1 at index 1)
        → quotient term: 2/1 * x^(3-1) = 2x²
        → q gets coefficient 2 at index 2
        → subtract (2x²) * (2 + x) from remainder:
             2x² * b = 2x² * (2 + x) = 4x² + 2x³
             aligned:  [0, 0, 4, 2]
             subtract: [5, 1, 3-4, 2-2] = [5, 1, -1, 0] → normalize → [5, 1, -1]

Step 2: remainder = [5, 1, -1], degree = 2
        2 >= 1, continue.
        leading term: -x²  (coeff -1 at index 2)
        → quotient term: -1/1 * x^(2-1) = -x
        → q gets coefficient -1 at index 1
        → subtract (-x) * (2 + x):
             -x * b = -2x - x²  = [0, -2, -1]
             subtract: [5-0, 1-(-2), -1-(-1)] = [5, 3, 0] → [5, 3]

Step 3: remainder = [5, 3], degree = 1
        1 >= 1, continue.
        leading term: 3x  (coeff 3 at index 1)
        → quotient term: 3/1 * x^(1-1) = 3
        → q gets coefficient 3 at index 0
        → subtract 3 * (2 + x):
             3 * b = [6, 3]
             subtract: [5-6, 3-3] = [-1, 0] → [-1]

Step 4: remainder = [-1], degree = 0
        0 < 1 = degree(b). STOP.

Result: quotient q = [3, -1, 2]  (3 - x + 2x²)
        remainder r = [-1]        (-1)

Verify: (x + 2)(3 - x + 2x²) + (-1)
     = 3x - x² + 2x³ + 6 - 2x + 4x²  - 1
     = 5 + x + 3x² + 2x³  ✓
```

### evaluate — Horner's Method

**Naive evaluation** of `a₀ + a₁x + a₂x² + ... + aₙxⁿ` requires n additions
and n multiplications (plus n exponentiations).

**Horner's method** rewrites the polynomial in nested form:

```
a₀ + x(a₁ + x(a₂ + x(... + x·aₙ)...))
```

This requires only n additions and n multiplications — no powers at all.

**Example**: Evaluate `3 + x + 2x²` at `x = 4`:

```
Coefficients in little-endian order: [3, 1, 2]
Horner reads from the HIGH end downward:

Start with aₙ = 2
Step 1: acc = 2·4 + 1 = 9
Step 2: acc = 9·4 + 3 = 39

Result: 39

Verify: 3 + 4 + 2·16 = 3 + 4 + 32 = 39  ✓
```

The pseudocode (iterating from high degree to low):

```
acc = 0
for i from degree(p) downto 0:
    acc = acc * x + p[i]
return acc
```

### gcd — Euclidean Algorithm

The **greatest common divisor** of two polynomials `a` and `b` is the
highest-degree monic polynomial that divides both with zero remainder.

It uses exactly the same Euclidean algorithm as integer GCD, with polynomial
mod in place of integer mod:

```
gcd(a, b):
    while b ≠ zero:
        a, b = b, a mod b
    return normalize(a)
```

**Example**: gcd(`[6, 7, 1]` = 6 + 7x + x², `[6, 5, 1]` = 6 + 5x + x²):

```
Round 1: a mod b
    a = [6, 7, 1]  =  (x+1)(x+6)
    b = [6, 5, 1]  =  (x+2)(x+3)
    a mod b → [2] (constant remainder 2)

Round 2: a = [6, 5, 1], b = [2]
    b is degree 0, so a mod b uses each coefficient mod 2... wait.
    Actually for real-number polynomials:
    a mod b where b is constant [2] → remainder is 0 (any poly is divisible by constant)
    so a mod [2] = [0] → normalize → []

Round 3: b = []  → stop.
Return normalize([2]) = [2]

The GCD is the constant 2, meaning (x+1)(x+6) and (x+2)(x+3) share no
common factor aside from constants. This is correct.
```

---

## Interface Contract

| Function | Signature | Returns | Throws |
|----------|-----------|---------|--------|
| `normalize(p)` | `(Polynomial) → Polynomial` | p with trailing zeros removed | — |
| `degree(p)` | `(Polynomial) → int` | highest non-zero index, or -1 | — |
| `zero()` | `() → Polynomial` | `[]` | — |
| `one()` | `() → Polynomial` | `[1]` | — |
| `add(a, b)` | `(Polynomial, Polynomial) → Polynomial` | normalized sum | — |
| `subtract(a, b)` | `(Polynomial, Polynomial) → Polynomial` | normalized difference | — |
| `multiply(a, b)` | `(Polynomial, Polynomial) → Polynomial` | normalized product | — |
| `divmod(a, b)` | `(Polynomial, Polynomial) → [Polynomial, Polynomial]` | `[quotient, remainder]` | b = zero |
| `divide(a, b)` | `(Polynomial, Polynomial) → Polynomial` | quotient only | b = zero |
| `mod(a, b)` | `(Polynomial, Polynomial) → Polynomial` | remainder only | b = zero |
| `evaluate(p, x)` | `(Polynomial, number) → number` | Horner evaluation | — |
| `gcd(a, b)` | `(Polynomial, Polynomial) → Polynomial` | GCD polynomial | — |

---

## Edge Cases

| Situation | Result |
|-----------|--------|
| `degree([])` | -1 |
| `degree([0])` | -1 (normalize strips to `[]`) |
| `add([], p)` | `p` |
| `multiply([], p)` | `[]` (zero times anything is zero) |
| `divmod(a, [])` | **error** — division by zero polynomial |
| `evaluate([], x)` | 0 |
| `gcd(p, [])` | `p` (GCD with zero is the other polynomial) |
| Trailing zeros | Always stripped by normalize |

---

## Backend Matrix

| Language | Package | Status |
|----------|---------|--------|
| TypeScript | `@coding-adventures/polynomial` | MA00 v0.1.0 |
| Python | `coding-adventures-polynomial` | MA00 v0.1.0 |
| Ruby | `coding_adventures_polynomial` | MA00 v0.1.0 |
| Go | `github.com/adhithyan15/coding-adventures/code/packages/go/polynomial` | MA00 v0.1.0 |

---

## Roadmap

- **MA01** — `gf256`: Galois Field GF(2^8) arithmetic using log/antilog tables.
  Polynomial arithmetic modulo the primitive polynomial `x^8 + x^4 + x^3 + x^2 + 1`.
- **MA02** — `reed-solomon`: Reed-Solomon encoding and decoding.
  Uses MA00 for polynomial arithmetic and MA01 for coefficient arithmetic.
