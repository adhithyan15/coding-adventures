"""Exact-rational diagnostics for VER04p §11.1.1.

This file checks a handful of explicit domain-classification and boundary
witnesses. It decodes only finite IEEE encodings into ``Fraction`` values; it
does not implement a rounding operation, softfloat evaluator, native-float
claim, or source/compiler proof.
"""

from __future__ import annotations

from fractions import Fraction


def _finite_value(bits: int, width: int) -> Fraction:
    """Decode one finite positive/negative F32 or F64 encoding exactly."""
    if width == 32:
        fraction_bits, exponent_bits, bias = 23, 8, 127
    elif width == 64:
        fraction_bits, exponent_bits, bias = 52, 11, 1023
    else:
        raise ValueError(f"unsupported width: {width}")

    bit_count = 1 + exponent_bits + fraction_bits
    if bits < 0 or bits >= 1 << bit_count:
        raise ValueError("encoding is outside the requested width")

    sign = -1 if bits >> (bit_count - 1) else 1
    exponent_mask = (1 << exponent_bits) - 1
    exponent = (bits >> fraction_bits) & exponent_mask
    fraction = bits & ((1 << fraction_bits) - 1)
    if exponent == exponent_mask:
        raise ValueError("diagnostic expects a finite encoding")

    if exponent == 0:
        significand = fraction
        power = 1 - bias - fraction_bits
    else:
        significand = (1 << fraction_bits) | fraction
        power = exponent - bias - fraction_bits

    numerator = sign * significand
    if power >= 0:
        return Fraction(numerator << power, 1)
    return Fraction(numerator, 1 << -power)


def f32_value(bits: int) -> Fraction:
    return _finite_value(bits, 32)


def f64_value(bits: int) -> Fraction:
    return _finite_value(bits, 64)


def require_exact_edge_value(value: Fraction, width: int, bits: int) -> None:
    """Fail if an exact source is mislabeled as a target-width float value."""
    decoded = _finite_value(bits, width)
    if Fraction(value) != decoded:
        raise ValueError(
            f"source {Fraction(value)} is not the value encoded by F{width} bits "
            f"0x{bits:0{width // 4}x} ({decoded})"
        )


def require_lower_even_tie(
    lower_bits: int, upper_bits: int, exact_midpoint: Fraction, width: int
) -> None:
    """Check an explicitly supplied adjacent-pair midpoint and even-lower bit."""
    if upper_bits != lower_bits + 1:
        raise ValueError("the witness encodings are not adjacent in this interval")
    lower = _finite_value(lower_bits, width)
    upper = _finite_value(upper_bits, width)
    if exact_midpoint - lower != upper - exact_midpoint:
        raise ValueError("the supplied rational is not the exact midpoint")
    if lower_bits & 1:
        raise ValueError("the lower encoding is not the even tie target")


def dedupe_final_bits(patterns: list[int]) -> list[int]:
    """Remove duplicate final encodings after width conversion, keeping first."""
    seen: set[int] = set()
    result: list[int] = []
    for bits in patterns:
        if bits not in seen:
            seen.add(bits)
            result.append(bits)
    return result


def require_unique_final_bits(patterns: list[int]) -> None:
    if len(patterns) != len(set(patterns)):
        raise ValueError("duplicate final bit pattern")


def verify_witnesses() -> dict[str, str]:
    """Validate only the concrete rational/encoding examples in VER04p."""
    f32_integer_lower = 0x4B800000
    f32_integer_upper = 0x4B800001
    f32_integer_midpoint = Fraction((1 << 24) + 1)
    require_exact_edge_value(Fraction(1 << 24), 32, f32_integer_lower)
    require_lower_even_tie(
        f32_integer_lower, f32_integer_upper, f32_integer_midpoint, 32
    )

    f64_integer_lower = 0x4340000000000000
    f64_integer_upper = 0x4340000000000001
    f64_integer_midpoint = Fraction((1 << 53) + 1)
    require_exact_edge_value(Fraction(1 << 53), 64, f64_integer_lower)
    require_lower_even_tie(
        f64_integer_lower, f64_integer_upper, f64_integer_midpoint, 64
    )

    f32_one = 0x3F800000
    f32_next_one = 0x3F800001
    f32_narrowing_midpoint = Fraction((1 << 24) + 1, 1 << 24)
    f64_narrowing_midpoint = 0x3FF0000010000000
    require_exact_edge_value(f32_narrowing_midpoint, 64, f64_narrowing_midpoint)
    require_lower_even_tie(
        f32_one, f32_next_one, f32_narrowing_midpoint, 32
    )

    f64_one = 0x3FF0000000000000
    f64_next_one = 0x3FF0000000000001
    f64_midpoint = Fraction((1 << 53) + 1, 1 << 53)
    f64_lower_value = f64_value(f64_one)
    f64_upper_value = f64_value(f64_next_one)
    if f64_midpoint - f64_lower_value != f64_upper_value - f64_midpoint:
        raise ValueError("binary64 witness is not the exact midpoint")
    if f64_next_one != f64_one + 1:
        raise ValueError("binary64 midpoint endpoints are not adjacent encodings")

    f32_max_finite = 0x7F7FFFFF
    f32_max_value = f32_value(f32_max_finite)
    if f32_max_value != (1 << 128) - (1 << 104):
        raise ValueError("largest finite F32 rational witness changed")
    if f32_max_value <= (1 << 127) - 1 or f32_max_value >= 1 << 128:
        raise ValueError("F32 witness does not establish a 128-bit magnitude")

    if f32_value(0x7F000000) != 1 << 127:
        raise ValueError("positive 2^127 F32 encoding changed")
    if f32_value(0xFF000000) != -(1 << 127):
        raise ValueError("negative 2^127 F32 encoding changed")

    f64_max_finite = 0x7FEFFFFFFFFFFFFF
    if f64_value(f64_max_finite) != (1 << 1024) - (1 << 971):
        raise ValueError("largest finite F64 rational witness changed")

    minsub = Fraction(1, 1 << 1074)
    one_tap = 2 * minsub
    if one_tap / one_tap != 1:
        raise ValueError("one-tap exact normalized gain is not one")
    reciprocal = 1 / one_tap
    f64_max = Fraction((1 << 53) - 1) * (1 << 971)
    if reciprocal != 1 << 1073 or reciprocal <= f64_max:
        raise ValueError("one-tap reciprocal boundary witness changed")

    # The representable lower neighbor and the documented integer conversion
    # witness are distinct exact sources but both finish at the lower F32 bits.
    # This checks one named tie, not a general rational-to-float rounder.
    f32_integer_sources = [Fraction(1 << 24), Fraction((1 << 24) + 1)]
    if f32_integer_sources[0] == f32_integer_sources[1]:
        raise ValueError("the pre-rounding candidate values must be distinct")
    require_exact_edge_value(f32_integer_sources[0], 32, f32_integer_lower)
    require_lower_even_tie(
        f32_integer_lower,
        f32_integer_upper,
        f32_integer_sources[1],
        32,
    )
    duplicate_f32_final_bits = [f32_integer_lower, f32_integer_lower]
    unique_f32 = dedupe_final_bits(duplicate_f32_final_bits)
    require_unique_final_bits(unique_f32)
    if unique_f32 != [f32_integer_lower]:
        raise ValueError("first-occurrence de-duplication changed")

    return {
        "F32 integer tie": "2^24+1 -> lower-even 0x4b800000",
        "F64 integer tie": "2^53+1 -> lower-even 0x4340000000000000",
        "F64 to F32 tie": "0x3ff0000010000000 -> 0x3f800000",
        "F64 exact midpoint": "1+2^-53 is between adjacent F64 encodings",
        "F32 exact integer range": "largest finite needs 128 magnitude bits",
        "F64 exact integer range": "largest finite needs 1024 magnitude bits",
        "subnormal normalization": "h/h=1 while 1/h is out of finite F64 range",
        "post-rounding de-dup": "2^24 and 2^24+1 finish at one F32 bit pattern",
    }


def main() -> None:
    for name, result in verify_witnesses().items():
        print(f"PASS {name}: {result}")


if __name__ == "__main__":
    main()
