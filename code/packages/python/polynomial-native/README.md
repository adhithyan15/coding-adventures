# polynomial-native

A Rust-backed Python native extension for polynomial arithmetic over `f64`
coefficients. Wraps the `polynomial` Rust crate via `python-bridge` — zero
third-party dependencies, no PyO3.

## What is a polynomial?

A polynomial is a mathematical expression like `3 + 2x + x²`. We represent it
as a Python `list[float]` where the **array index equals the degree** of each
term (little-endian, lowest degree first):

```text
[3.0, 2.0, 1.0]  →  3 + 2x + x²
[0.0, 0.0, 1.0]  →  x²
[1.0]            →  the constant 1
[]               →  the zero polynomial
```

## Where does it fit?

```
code/packages/rust/polynomial   ← core Rust implementation
code/packages/python/polynomial-native  ← this package (Python bindings)
```

The native extension produces `polynomial_native.so` which Python imports
directly. All arithmetic runs in Rust; only function call boundaries cross
the Python/Rust barrier.

## Usage

```python
import polynomial_native as pn

# Polynomial 3 + 0x + x^2
p = [3.0, 0.0, 1.0]

# Evaluate at x=2: 3 + 0*2 + 1*4 = 7
pn.evaluate(p, 2.0)  # → 7.0

# Add two polynomials
pn.add([1.0, 2.0], [3.0, 4.0])  # → [4.0, 6.0]

# Multiply
pn.multiply([1.0, 2.0], [3.0, 4.0])  # → [3.0, 10.0, 8.0]

# Long division
quot, rem = pn.divmod_poly([2.0, 0.0, 1.0], [1.0, 1.0])

# GCD
pn.gcd([2.0, -3.0, 1.0], [-1.0, 1.0])
```

## Functions

| Function | Description |
|---|---|
| `normalize(poly)` | Strip trailing near-zero coefficients |
| `degree(poly)` | Degree (index of highest non-zero term) |
| `zero()` | Zero polynomial `[0.0]` |
| `one()` | Identity polynomial `[1.0]` |
| `add(a, b)` | Polynomial addition |
| `subtract(a, b)` | Polynomial subtraction |
| `multiply(a, b)` | Polynomial multiplication |
| `divmod_poly(a, b)` | Long division → `(quotient, remainder)` |
| `divide(a, b)` | Quotient only |
| `modulo(a, b)` | Remainder only |
| `evaluate(poly, x)` | Evaluate at `x` (Horner's method) |
| `gcd(a, b)` | GCD via Euclidean algorithm |

Division functions raise `ValueError` if the divisor is the zero polynomial.

## Building

```bash
cargo build --release
cp target/release/libpolynomial_native.dylib src/polynomial_native/polynomial_native.so
PYTHONPATH=src python -m pytest tests/ -v
```

Or use the `BUILD` file with the repo's build tool.
