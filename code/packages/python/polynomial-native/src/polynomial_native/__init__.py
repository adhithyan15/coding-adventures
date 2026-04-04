"""
polynomial_native -- Native Extension (Rust-backed via python-bridge)
=====================================================================

A Rust-backed implementation of polynomial arithmetic over f64 coefficients.
Polynomials are represented as ``list[float]`` where the array index equals
the degree of that term's coefficient (little-endian / lowest degree first):

    [3.0, 0.0, 2.0]  →  3 + 0x + 2x²  =  3 + 2x²
    [1.0, 2.0, 3.0]  →  1 + 2x + 3x²
    []               →  the zero polynomial

All arithmetic runs in Rust. Only the function call boundary crosses between
Python and Rust, making this a fast drop-in for the pure-Python polynomial
package.

Functions
---------
normalize(poly)              Strip trailing near-zero coefficients.
degree(poly)                 Degree of the polynomial (int).
zero()                       The zero polynomial [0.0].
one()                        The multiplicative identity [1.0].
add(a, b)                    Add two polynomials.
subtract(a, b)               Subtract b from a.
multiply(a, b)               Multiply two polynomials.
divmod_poly(dividend, div)   Long division → (quotient, remainder) tuple.
divide(a, b)                 Quotient only.
modulo(a, b)                 Remainder only.
evaluate(poly, x)            Evaluate at x using Horner's method.
gcd(a, b)                    GCD via Euclidean algorithm.
"""

# The native .so/.dylib/.pyd is compiled from Rust and placed in this
# directory by the build process. It exports all free functions via
# PyInit_polynomial_native.
from polynomial_native.polynomial_native import (  # type: ignore[import]
    normalize,
    degree,
    zero,
    one,
    add,
    subtract,
    multiply,
    divmod_poly,
    divide,
    modulo,
    evaluate,
    gcd,
)

__all__ = [
    "normalize",
    "degree",
    "zero",
    "one",
    "add",
    "subtract",
    "multiply",
    "divmod_poly",
    "divide",
    "modulo",
    "evaluate",
    "gcd",
]
