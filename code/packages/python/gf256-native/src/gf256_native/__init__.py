"""
gf256_native -- Native Extension (Rust-backed via python-bridge)
================================================================

A Rust-backed implementation of GF(2^8) (Galois Field with 256 elements)
arithmetic. All operations run in Rust; only function call boundaries cross
between Python and Rust.

## What is GF(256)?

GF(2^8) is a finite field with exactly 256 elements: the bytes 0–255.
The arithmetic is not ordinary integer arithmetic:

  - Addition is XOR:  ``add(0x53, 0xCA) == 0x99``
  - Subtraction equals addition (characteristic 2): ``subtract(a, b) == add(a, b)``
  - Multiplication uses log/antilog lookup tables built from a primitive generator
  - Division: multiply by the inverse

GF(256) powers many important algorithms: Reed-Solomon error correction (QR
codes, CDs, DVDs), and AES encryption.

Functions
---------
add(a, b)         XOR addition.
subtract(a, b)    XOR subtraction (same as add).
multiply(a, b)    Table-based multiplication.
divide(a, b)      Field division. Raises ValueError if b == 0.
power(base, exp)  Exponentiation in the field.
inverse(a)        Multiplicative inverse. Raises ValueError if a == 0.

Constants
---------
ZERO                 = 0      Additive identity.
ONE                  = 1      Multiplicative identity.
PRIMITIVE_POLYNOMIAL = 0x11D  Irreducible polynomial x^8+x^4+x^3+x^2+1.
"""

# The native .so/.dylib/.pyd is compiled from Rust and placed in this
# directory by the build process. It exports all functions and constants
# via PyInit_gf256_native.
from gf256_native.gf256_native import (  # type: ignore[import]
    add,
    subtract,
    multiply,
    divide,
    power,
    inverse,
    ZERO,
    ONE,
    PRIMITIVE_POLYNOMIAL,
)

__all__ = [
    "add",
    "subtract",
    "multiply",
    "divide",
    "power",
    "inverse",
    "ZERO",
    "ONE",
    "PRIMITIVE_POLYNOMIAL",
]
