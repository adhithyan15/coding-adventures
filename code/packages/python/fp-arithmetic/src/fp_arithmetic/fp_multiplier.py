"""Floating-point multiplication — built from logic gates.

=== How FP multiplication works ===

Floating-point multiplication is actually simpler than addition! That's because
you don't need to align mantissas — the exponents just add together.

In scientific notation:
    (1.5 x 10^3) x (2.0 x 10^2) = (1.5 x 2.0) x 10^(3+2) = 3.0 x 10^5

The same principle applies in binary:
    (-1)^s1 x 1.m1 x 2^e1  *  (-1)^s2 x 1.m2 x 2^e2
    = (-1)^(s1 XOR s2) x (1.m1 x 1.m2) x 2^(e1 + e2)

=== The four steps of FP multiplication ===

    Step 1: Result sign = XOR of input signs
            Positive x Positive = Positive (0 XOR 0 = 0)
            Positive x Negative = Negative (0 XOR 1 = 1)
            Negative x Negative = Positive (1 XOR 1 = 0)

    Step 2: Result exponent = exp_a + exp_b - bias
            We subtract the bias once because both exponents include it:
            true_exp_a = stored_a - bias
            true_exp_b = stored_b - bias
            true_result = true_a + true_b
            stored_result = true_result + bias = stored_a + stored_b - bias

    Step 3: Multiply mantissas using shift-and-add
            This is the core of the operation. For each bit of one mantissa,
            if that bit is 1, we add the other mantissa shifted by that position.
            The result is double-width (e.g., 48 bits for FP32's 24-bit mantissas).

    Step 4: Normalize and round (same as addition)

=== Shift-and-add multiplication ===

Binary multiplication works exactly like long multiplication you learned in
school, but simpler because each digit is only 0 or 1:

    1.101  (multiplicand = 1.625 in decimal)
  x 1.011  (multiplier   = 1.375 in decimal)
  -------
    1101   (1.101 x 1)     — multiplier bit 0 is 1, so add
   1101    (1.101 x 1)     — multiplier bit 1 is 1, so add (shifted left 1)
  0000     (1.101 x 0)     — multiplier bit 2 is 0, skip
 1101      (1.101 x 1)     — multiplier bit 3 is 1, so add (shifted left 3)
 ---------
 10.001111  = 2.234375 in decimal

Check: 1.625 x 1.375 = 2.234375 correct!

In hardware, each "if bit is 1, add shifted value" is an AND gate (to gate
the addition) followed by a ripple_carry_adder. Our implementation uses
Python integers for clarity, but the algorithm is the same.
"""

from __future__ import annotations

from fp_arithmetic._gates import XOR

from fp_arithmetic.formats import FloatBits, FloatFormat
from fp_arithmetic.ieee754 import (
    _bits_msb_to_int,
    _int_to_bits_msb,
    is_inf,
    is_nan,
    is_zero,
)


def fp_mul(a: FloatBits, b: FloatBits) -> FloatBits:
    """Multiply two floating-point numbers using logic gates.

    Implements the IEEE 754 multiplication algorithm:
    1. Handle special cases (NaN, Inf, Zero)
    2. XOR signs
    3. Add exponents, subtract bias
    4. Multiply mantissas (shift-and-add)
    5. Normalize and round

    === Worked example: 1.5 x 2.0 in FP32 ===

        1.5 = 1.1 x 2^0    -> sign=0, exp=127, mant=100...0
        2.0 = 1.0 x 2^1    -> sign=0, exp=128, mant=000...0

        Step 1: result_sign = 0 XOR 0 = 0 (positive)
        Step 2: result_exp = 127 + 128 - 127 = 128 (true exp = 1)
        Step 3: mantissa product:
                1.100...0 x 1.000...0 = 1.100...0 (trivial case)
        Step 4: Already normalized
        Result: 1.1 x 2^1 = 3.0 (correct!)

    Args:
        a: First operand as FloatBits.
        b: Second operand as FloatBits. Must use the same FloatFormat as a.

    Returns:
        The product as FloatBits in the same format.
    """
    fmt = a.fmt

    # ===================================================================
    # Step 0: Handle special cases
    # ===================================================================
    # IEEE 754 rules for multiplication:
    #   NaN x anything = NaN
    #   Inf x 0 = NaN
    #   Inf x finite = Inf (with appropriate sign)
    #   0 x finite = 0

    # Result sign: always XOR of input signs (even for special cases)
    result_sign = XOR(a.sign, b.sign)

    # NaN propagation
    if is_nan(a) or is_nan(b):
        return FloatBits(
            sign=0,
            exponent=[1] * fmt.exponent_bits,
            mantissa=[1] + [0] * (fmt.mantissa_bits - 1),
            fmt=fmt,
        )

    a_inf = is_inf(a)
    b_inf = is_inf(b)
    a_zero = is_zero(a)
    b_zero = is_zero(b)

    # Inf x 0 = NaN (undefined)
    if (a_inf and b_zero) or (b_inf and a_zero):
        return FloatBits(
            sign=0,
            exponent=[1] * fmt.exponent_bits,
            mantissa=[1] + [0] * (fmt.mantissa_bits - 1),
            fmt=fmt,
        )

    # Inf x anything = Inf
    if a_inf or b_inf:
        return FloatBits(
            sign=result_sign,
            exponent=[1] * fmt.exponent_bits,
            mantissa=[0] * fmt.mantissa_bits,
            fmt=fmt,
        )

    # Zero x anything = Zero
    if a_zero or b_zero:
        return FloatBits(
            sign=result_sign,
            exponent=[0] * fmt.exponent_bits,
            mantissa=[0] * fmt.mantissa_bits,
            fmt=fmt,
        )

    # ===================================================================
    # Step 1: Extract exponents and mantissas
    # ===================================================================
    exp_a = _bits_msb_to_int(a.exponent)
    exp_b = _bits_msb_to_int(b.exponent)
    mant_a = _bits_msb_to_int(a.mantissa)
    mant_b = _bits_msb_to_int(b.mantissa)

    # Add implicit leading 1 for normal numbers
    if exp_a != 0:
        mant_a = (1 << fmt.mantissa_bits) | mant_a
    else:
        exp_a = 1  # Denormal: true exponent = 1 - bias

    if exp_b != 0:
        mant_b = (1 << fmt.mantissa_bits) | mant_b
    else:
        exp_b = 1

    # ===================================================================
    # Step 2: Add exponents, subtract bias
    # ===================================================================
    #
    # result_exp = exp_a + exp_b - bias
    #
    # Why subtract bias? Both exp_a and exp_b include the bias:
    #   true_a = exp_a - bias
    #   true_b = exp_b - bias
    #   true_result = true_a + true_b = (exp_a - bias) + (exp_b - bias)
    #   stored_result = true_result + bias = exp_a + exp_b - bias
    #
    # In hardware, this is two ripple_carry_adder operations:
    #   1. Add the two exponents
    #   2. Subtract the bias

    result_exp = exp_a + exp_b - fmt.bias

    # ===================================================================
    # Step 3: Multiply mantissas (shift-and-add)
    # ===================================================================
    #
    # The mantissa product of two (mantissa_bits+1)-bit numbers produces
    # a (2*(mantissa_bits+1))-bit result.
    #
    # For FP32: 24-bit x 24-bit = 48-bit product.
    #
    # === Shift-and-add algorithm ===
    #
    # For each bit position i in the multiplier (mant_b):
    #   If bit i is 1:
    #     product += mant_a << i   (AND gate + adder)
    #   If bit i is 0:
    #     product += 0             (AND gate produces 0)
    #
    # In hardware, the AND gate acts as a "conditional add": AND(multiplier_bit, X)
    # produces X when the bit is 1, and 0 when the bit is 0.
    #
    # We use Python integer multiplication here because the shift-and-add
    # at the Python level would be identical in result but much slower.
    # The important thing is understanding that hardware does it as shift-and-add.

    product = mant_a * mant_b
    # The product has up to 2*(mantissa_bits+1) bits

    # ===================================================================
    # Step 4: Normalize
    # ===================================================================
    #
    # The product of two 1.xxx numbers is between 1.0 and 3.999..., so
    # the product is either 1x.xxx or 01.xxx in binary.
    #
    # If the product's MSB is at position 2*(mantissa_bits+1)-1, we need
    # to shift right by 1 and increment the exponent.

    # The "normal" position for the product: the leading 1 should be at
    # bit position 2*mantissa_bits (from counting the two implicit 1s)
    product_width = 2 * (fmt.mantissa_bits + 1)
    leading_pos = product.bit_length() - 1
    normal_pos = 2 * fmt.mantissa_bits  # Where leading 1 should be

    if leading_pos > normal_pos:
        # Product overflowed: 1x.xxx form, shift right
        extra = leading_pos - normal_pos
        result_exp += extra
    elif leading_pos < normal_pos:
        # Product is smaller than expected (happens with denormals)
        deficit = normal_pos - leading_pos
        result_exp -= deficit

    # ===================================================================
    # Step 5: Round to nearest even
    # ===================================================================
    #
    # We need to reduce the product from ~48 bits to 24 bits (for FP32).
    # The bits below the mantissa field determine rounding.

    # How many bits are below the mantissa in the product?
    # product has its leading 1 at position `leading_pos`
    # We want mantissa_bits after the leading 1
    # So the "round point" is at position (leading_pos - mantissa_bits)
    round_pos = leading_pos - fmt.mantissa_bits

    if round_pos > 0:
        # Extract guard, round, sticky bits for rounding
        guard = (product >> (round_pos - 1)) & 1
        if round_pos >= 2:
            round_bit = (product >> (round_pos - 2)) & 1
            sticky = 1 if (product & ((1 << (round_pos - 2)) - 1)) != 0 else 0
        else:
            round_bit = 0
            sticky = 0

        # Truncate to mantissa width + 1 (including implicit 1)
        result_mant = product >> round_pos

        # Apply rounding
        if guard == 1:
            if round_bit == 1 or sticky == 1:
                result_mant += 1
            elif (result_mant & 1) == 1:
                result_mant += 1

        # Check if rounding caused mantissa overflow
        if result_mant >= (1 << (fmt.mantissa_bits + 1)):
            result_mant >>= 1
            result_exp += 1
    elif round_pos == 0:
        result_mant = product
    else:
        # Product is very small, shift left
        result_mant = product << (-round_pos)

    # ===================================================================
    # Step 6: Handle exponent overflow/underflow
    # ===================================================================
    max_exp = (1 << fmt.exponent_bits) - 1

    if result_exp >= max_exp:
        # Overflow to infinity
        return FloatBits(
            sign=result_sign,
            exponent=[1] * fmt.exponent_bits,
            mantissa=[0] * fmt.mantissa_bits,
            fmt=fmt,
        )

    if result_exp <= 0:
        # Denormal or underflow
        if result_exp < -(fmt.mantissa_bits):
            return FloatBits(
                sign=result_sign,
                exponent=[0] * fmt.exponent_bits,
                mantissa=[0] * fmt.mantissa_bits,
                fmt=fmt,
            )
        # Shift mantissa right to make it denormal
        shift = 1 - result_exp
        result_mant >>= shift
        result_exp = 0

    # ===================================================================
    # Step 7: Pack the result
    # ===================================================================
    # Remove the implicit leading 1 (if normal)
    if result_exp > 0:
        result_mant &= (1 << fmt.mantissa_bits) - 1

    return FloatBits(
        sign=result_sign,
        exponent=_int_to_bits_msb(result_exp, fmt.exponent_bits),
        mantissa=_int_to_bits_msb(result_mant, fmt.mantissa_bits),
        fmt=fmt,
    )
