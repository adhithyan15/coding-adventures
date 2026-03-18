"""Fused Multiply-Add and format conversion.

=== What is FMA (Fused Multiply-Add)? ===

FMA computes a * b + c with only ONE rounding step at the end. Compare:

    Without FMA (separate operations):
        temp = fp_mul(a, b)   # round #1 (loses precision)
        result = fp_add(temp, c)  # round #2 (loses more precision)

    With FMA:
        result = fp_fma(a, b, c)  # round only once!

=== Why FMA matters for ML ===

In machine learning, the dominant computation is the dot product:
    result = sum(a[i] * w[i] for i in range(N))

Each multiply-add in the sum is a potential FMA. By rounding only once per
operation instead of twice, FMA gives more accurate gradients during training.
This seemingly small improvement compounds over millions of operations.

Every modern processor has FMA:
- Intel Haswell (2013): FMA3 instruction (AVX2)
- NVIDIA GPUs: native FMA in CUDA cores
- Google TPU: the MAC (Multiply-Accumulate) unit IS an FMA
- Apple M-series: FMA in both CPU and Neural Engine

=== Algorithm ===

    Step 1: Multiply a * b with FULL precision (no rounding!)
            For FP32: 24-bit x 24-bit = 48-bit product (no information lost)

    Step 2: Align c's mantissa to the product's exponent
            (same as the alignment step in fp_add)

    Step 3: Add the full-precision product and aligned c

    Step 4: Normalize and round ONCE

The key insight is Step 1: by keeping the full 48-bit product without rounding,
we preserve all the information for the final result. The separate mul+add
approach throws away bits in the intermediate rounding, which can never be
recovered.

=== Format conversion ===

This module also provides fp_convert() for converting between FP32, FP16, and
BF16. Format conversion is essentially re-encoding: decode the value, then
encode in the target format (possibly losing precision).
"""

from __future__ import annotations

from fp_arithmetic._gates import XOR

from fp_arithmetic.formats import FloatBits, FloatFormat
from fp_arithmetic.ieee754 import (
    _bits_msb_to_int,
    _int_to_bits_msb,
    bits_to_float,
    float_to_bits,
    is_inf,
    is_nan,
    is_zero,
)


def fp_fma(a: FloatBits, b: FloatBits, c: FloatBits) -> FloatBits:
    """Fused multiply-add: compute a * b + c with single rounding.

    === Worked example: FMA(1.5, 2.0, 0.25) in FP32 ===

        a = 1.5 = 1.1 x 2^0    (exp=127, mant=1.100...0)
        b = 2.0 = 1.0 x 2^1    (exp=128, mant=1.000...0)
        c = 0.25 = 1.0 x 2^-2  (exp=125, mant=1.000...0)

        Step 1: Full-precision multiply
                1.100...0 x 1.000...0 = 1.100...0 (48-bit, no rounding)
                Product exponent: 127 + 128 - 127 = 128 (true exp = 1)
                So product = 1.1 x 2^1 = 3.0

        Step 2: Align c to product's exponent
                c = 1.0 x 2^-2, product exponent = 128
                Shift c right by 128 - 125 = 3 positions
                c_aligned = 0.001 x 2^1

        Step 3: Add
                1.100 x 2^1 + 0.001 x 2^1 = 1.101 x 2^1

        Step 4: Normalize and round
                Already normalized, result = 1.101 x 2^1 = 3.25
                Check: 1.5 * 2.0 + 0.25 = 3.0 + 0.25 = 3.25 correct!

    Args:
        a: First multiplicand.
        b: Second multiplicand.
        c: Addend.

    Returns:
        a * b + c as FloatBits, with only one rounding step.
    """
    fmt = a.fmt

    # ===================================================================
    # Step 0: Handle special cases
    # ===================================================================
    # NaN propagation
    if is_nan(a) or is_nan(b) or is_nan(c):
        return FloatBits(
            sign=0,
            exponent=[1] * fmt.exponent_bits,
            mantissa=[1] + [0] * (fmt.mantissa_bits - 1),
            fmt=fmt,
        )

    a_inf = is_inf(a)
    b_inf = is_inf(b)
    c_inf = is_inf(c)
    a_zero = is_zero(a)
    b_zero = is_zero(b)

    # Inf * 0 = NaN
    if (a_inf and b_zero) or (b_inf and a_zero):
        return FloatBits(
            sign=0,
            exponent=[1] * fmt.exponent_bits,
            mantissa=[1] + [0] * (fmt.mantissa_bits - 1),
            fmt=fmt,
        )

    product_sign = XOR(a.sign, b.sign)

    # Inf * finite + c
    if a_inf or b_inf:
        if c_inf and product_sign != c.sign:
            # Inf + (-Inf) = NaN
            return FloatBits(
                sign=0,
                exponent=[1] * fmt.exponent_bits,
                mantissa=[1] + [0] * (fmt.mantissa_bits - 1),
                fmt=fmt,
            )
        return FloatBits(
            sign=product_sign,
            exponent=[1] * fmt.exponent_bits,
            mantissa=[0] * fmt.mantissa_bits,
            fmt=fmt,
        )

    # a * b = 0, result is just c (but we need to handle 0 + 0 sign)
    if a_zero or b_zero:
        if is_zero(c):
            # 0 + 0: sign depends on rounding mode, default to +0
            # unless both are negative
            from fp_arithmetic._gates import AND
            result_sign = AND(product_sign, c.sign)
            return FloatBits(
                sign=result_sign,
                exponent=[0] * fmt.exponent_bits,
                mantissa=[0] * fmt.mantissa_bits,
                fmt=fmt,
            )
        return c

    # c is Inf
    if c_inf:
        return c

    # ===================================================================
    # Step 1: Multiply a * b with full precision (no rounding!)
    # ===================================================================
    exp_a = _bits_msb_to_int(a.exponent)
    exp_b = _bits_msb_to_int(b.exponent)
    mant_a = _bits_msb_to_int(a.mantissa)
    mant_b = _bits_msb_to_int(b.mantissa)

    # Add implicit leading 1 for normal numbers
    if exp_a != 0:
        mant_a = (1 << fmt.mantissa_bits) | mant_a
    else:
        exp_a = 1

    if exp_b != 0:
        mant_b = (1 << fmt.mantissa_bits) | mant_b
    else:
        exp_b = 1

    # Full-precision product: no truncation, no rounding!
    # For FP32: 24-bit x 24-bit = up to 48-bit result
    product = mant_a * mant_b

    # Product exponent (before normalization)
    product_exp = exp_a + exp_b - fmt.bias

    # Normalize the product
    # Product of two 1.xxx numbers: leading 1 is at bit 2*mantissa_bits or 2*mantissa_bits+1
    product_leading = product.bit_length() - 1
    normal_product_pos = 2 * fmt.mantissa_bits

    if product_leading > normal_product_pos:
        product_exp += product_leading - normal_product_pos
    elif product_leading < normal_product_pos:
        product_exp -= normal_product_pos - product_leading

    # ===================================================================
    # Step 2: Align c's mantissa to the product's exponent
    # ===================================================================
    exp_c = _bits_msb_to_int(c.exponent)
    mant_c = _bits_msb_to_int(c.mantissa)

    if exp_c != 0:
        mant_c = (1 << fmt.mantissa_bits) | mant_c
    else:
        exp_c = 1

    # The product has (product_leading + 1) bits.
    # c has (mantissa_bits + 1) bits.
    # We need to align them to the same exponent.

    # Scale c's mantissa to match product's bit width
    # c_shifted represents c at product's scale
    exp_diff = product_exp - exp_c

    # Use a wide enough workspace for the addition
    # We need to fit both the product and c
    product_width = product_leading + 1

    if exp_diff >= 0:
        # Product exponent >= c exponent: shift c right
        # First, scale c to the same bit-width as product by shifting left
        # to align the "implicit 1" positions, then shift right by exp_diff
        c_scale_shift = product_leading - fmt.mantissa_bits
        if c_scale_shift >= 0:
            c_aligned = mant_c << c_scale_shift
        else:
            c_aligned = mant_c >> (-c_scale_shift)
        c_aligned >>= exp_diff
        result_exp = product_exp
    else:
        # c exponent > product exponent: shift product right
        c_scale_shift = product_leading - fmt.mantissa_bits
        if c_scale_shift >= 0:
            c_aligned = mant_c << c_scale_shift
        else:
            c_aligned = mant_c >> (-c_scale_shift)
        product >>= (-exp_diff)
        result_exp = exp_c

    # ===================================================================
    # Step 3: Add product and c
    # ===================================================================
    if product_sign == c.sign:
        result_mant = product + c_aligned
        result_sign = product_sign
    else:
        if product >= c_aligned:
            result_mant = product - c_aligned
            result_sign = product_sign
        else:
            result_mant = c_aligned - product
            result_sign = c.sign

    # Handle zero result
    if result_mant == 0:
        return FloatBits(
            sign=0,
            exponent=[0] * fmt.exponent_bits,
            mantissa=[0] * fmt.mantissa_bits,
            fmt=fmt,
        )

    # ===================================================================
    # Step 4: Normalize and round ONCE
    # ===================================================================
    # Find the leading 1 in the result
    result_leading = result_mant.bit_length() - 1
    # The target position for the leading 1
    target_pos = product_leading if product_leading > fmt.mantissa_bits else fmt.mantissa_bits

    if result_leading > target_pos:
        shift = result_leading - target_pos
        result_exp += shift
    elif result_leading < target_pos:
        shift_needed = target_pos - result_leading
        result_exp -= shift_needed

    # Now round to mantissa_bits precision
    result_leading = result_mant.bit_length() - 1
    round_pos = result_leading - fmt.mantissa_bits

    if round_pos > 0:
        guard = (result_mant >> (round_pos - 1)) & 1
        if round_pos >= 2:
            round_bit = (result_mant >> (round_pos - 2)) & 1
            sticky = 1 if (result_mant & ((1 << (round_pos - 2)) - 1)) != 0 else 0
        else:
            round_bit = 0
            sticky = 0

        result_mant >>= round_pos

        # Round to nearest even
        if guard == 1:
            if round_bit == 1 or sticky == 1:
                result_mant += 1
            elif (result_mant & 1) == 1:
                result_mant += 1

        # Check rounding overflow
        if result_mant >= (1 << (fmt.mantissa_bits + 1)):
            result_mant >>= 1
            result_exp += 1
    elif round_pos < 0:
        result_mant <<= (-round_pos)

    # Handle exponent overflow/underflow
    max_exp = (1 << fmt.exponent_bits) - 1

    if result_exp >= max_exp:
        return FloatBits(
            sign=result_sign,
            exponent=[1] * fmt.exponent_bits,
            mantissa=[0] * fmt.mantissa_bits,
            fmt=fmt,
        )

    if result_exp <= 0:
        if result_exp < -(fmt.mantissa_bits):
            return FloatBits(
                sign=result_sign,
                exponent=[0] * fmt.exponent_bits,
                mantissa=[0] * fmt.mantissa_bits,
                fmt=fmt,
            )
        shift = 1 - result_exp
        result_mant >>= shift
        result_exp = 0

    # Remove implicit leading 1
    if result_exp > 0:
        result_mant &= (1 << fmt.mantissa_bits) - 1

    return FloatBits(
        sign=result_sign,
        exponent=_int_to_bits_msb(result_exp, fmt.exponent_bits),
        mantissa=_int_to_bits_msb(result_mant, fmt.mantissa_bits),
        fmt=fmt,
    )


# ---------------------------------------------------------------------------
# Format conversion: FP32 <-> FP16 <-> BF16
# ---------------------------------------------------------------------------


def fp_convert(bits: FloatBits, target_fmt: FloatFormat) -> FloatBits:
    """Convert a floating-point number from one format to another.

    === Why format conversion matters ===

    In ML pipelines, data frequently changes precision:
    - Training starts in FP32 (full precision)
    - Forward pass uses FP16 or BF16 (faster, less memory)
    - Gradients accumulated in FP32 (need precision)
    - Weights stored as BF16 on TPU

    Each conversion potentially loses precision (if going to a smaller format)
    or is exact (if going to a larger format).

    === FP32 -> BF16 conversion (trivially simple!) ===

    BF16 was designed so that conversion from FP32 is dead simple:
    just truncate the lower 16 bits! Both formats use the same 8-bit
    exponent with bias 127, so no exponent adjustment is needed.

        FP32: [sign(1)] [exponent(8)] [mantissa(23)]
        BF16: [sign(1)] [exponent(8)] [mantissa(7) ]
                                       ^^^^^^^^^^^ just take the top 7 of 23

    This is why Google chose this format for TPU: the conversion circuit
    is essentially free (just wires, no logic gates needed).

    Args:
        bits: The source FloatBits to convert.
        target_fmt: The target FloatFormat.

    Returns:
        The value in the target format (possibly with precision loss).

    Example:
        >>> fp32_val = float_to_bits(3.14, FP32)
        >>> bf16_val = fp_convert(fp32_val, BF16)
        >>> bits_to_float(bf16_val)  # Less precise
        3.140625
    """
    # Same format: no conversion needed
    if bits.fmt == target_fmt:
        return bits

    # Strategy: decode to Python float, then re-encode in target format.
    # This handles all the edge cases (denormals, rounding, overflow)
    # correctly by leveraging our existing encode/decode functions.
    #
    # A hardware implementation would directly manipulate the bit fields
    # (adjust exponent bias, truncate/extend mantissa), but for educational
    # purposes, the decode-then-encode approach is clearer and provably correct.
    value = bits_to_float(bits)
    return float_to_bits(value, target_fmt)
