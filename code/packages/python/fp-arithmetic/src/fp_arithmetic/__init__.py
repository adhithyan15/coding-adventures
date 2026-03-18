"""FP Arithmetic — IEEE 754 floating-point arithmetic from logic gates.

This package is the shared foundation for both the CPU stack (FPU) and all
three accelerator stacks (GPU/TPU/NPU). Every floating-point operation in
the computing stack ultimately passes through this layer.

Built entirely from logic gates (AND, OR, XOR, NOT) and ripple-carry adders.
"""

from fp_arithmetic.fma import fp_convert, fp_fma
from fp_arithmetic.formats import BF16, FP16, FP32, FloatBits, FloatFormat
from fp_arithmetic.fp_adder import fp_abs, fp_add, fp_compare, fp_neg, fp_sub
from fp_arithmetic.fp_multiplier import fp_mul
from fp_arithmetic.ieee754 import (
    bits_to_float,
    float_to_bits,
    is_denormalized,
    is_inf,
    is_nan,
    is_zero,
)

__all__ = [
    # Formats
    "FloatFormat",
    "FloatBits",
    "FP32",
    "FP16",
    "BF16",
    # Encoding/decoding
    "float_to_bits",
    "bits_to_float",
    # Special value detection
    "is_nan",
    "is_inf",
    "is_zero",
    "is_denormalized",
    # Arithmetic
    "fp_add",
    "fp_sub",
    "fp_mul",
    "fp_fma",
    # Utility
    "fp_neg",
    "fp_abs",
    "fp_compare",
    "fp_convert",
]
