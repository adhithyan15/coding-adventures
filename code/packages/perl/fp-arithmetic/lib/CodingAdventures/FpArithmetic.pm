package CodingAdventures::FpArithmetic;

# ============================================================================
# CodingAdventures::FpArithmetic — IEEE 754 single-precision arithmetic
# ============================================================================
#
# This module teaches IEEE 754 floating-point from the ground up.  We work
# with 32-bit integers whose bit patterns represent single-precision (FP32)
# floating-point values.  No actual hardware FP is used for the core
# encode/decode/add/mul operations — we manipulate bit fields by hand.
#
# === THE IEEE 754 FP32 BIT LAYOUT ===
#
#   Bit 31:     sign      (0 = positive, 1 = negative)
#   Bits 30-23: exponent  (8 bits, biased: stored = true_exponent + 127)
#   Bits 22-0:  mantissa  (23 bits, the fractional part after the implicit "1.")
#
#   Diagram:
#
#     3         2         1         0
#     1098 7654 3210 9876 5432 1098 7654 3210
#     SEEE EEEE EMMM MMMM MMMM MMMM MMMM MMMM
#     |         |                            |
#     sign      exponent                     mantissa
#
# === SPECIAL VALUES ===
#
#   Exponent   Mantissa   Meaning
#   --------   --------   -------
#   0xFF       != 0       NaN  (Not a Number: 0/0, sqrt(-1), etc.)
#   0xFF       == 0       +/- Infinity
#   0x00       == 0       +/- Zero
#   0x00       != 0       Denormalized (subnormal) number
#   other      any        Normal number
#
# === THE IMPLICIT LEADING BIT ===
#
# Normal numbers have an implicit leading 1 that is NOT stored.  The actual
# value of a normal FP32 is:
#
#   (-1)^sign  *  1.mantissa  *  2^(exponent - 127)
#
# This gives us 24 bits of precision (1 implicit + 23 stored) for free.
# For example, the mantissa field 0b10000...0 represents 1.1 in binary = 1.5.
#
# USAGE:
#
#   use CodingAdventures::FpArithmetic qw(
#       encode_f32 decode_f32
#       f32_add f32_mul
#       f32_to_string float_to_f32 f32_to_float
#   );
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(
    encode_f32 decode_f32
    f32_add f32_mul
    f32_to_string float_to_f32 f32_to_float
);

# ============================================================================
# CONSTANTS
# ============================================================================

# FP32 has 8 exponent bits, so the bias is 2^(8-1) - 1 = 127.
# True exponent = stored_exponent - BIAS.
use constant BIAS         => 127;
use constant MANTISSA_BITS => 23;
use constant EXP_BITS     => 8;
use constant MAX_EXP      => 0xFF;  # 255: all exponent bits set → NaN or Inf

# ============================================================================
# encode_f32 — pack (sign, exp, mantissa) into a 32-bit integer
# ============================================================================
#
# === Why bitmask and shift? ===
#
# The three fields of an FP32 number sit at fixed bit positions.  To pack
# them into a single 32-bit integer we shift each field to its correct
# position and OR them together:
#
#   bits = (sign << 31) | (exponent << 23) | mantissa
#
# We mask each field first to prevent overflow:
#
#   sign:     & 0x1      (1 bit)
#   exponent: & 0xFF     (8 bits)
#   mantissa: & 0x7FFFFF (23 bits)
#
# @param $sign     0 or 1
# @param $exp      stored (biased) exponent, 0..255
# @param $mantissa 23-bit integer, 0..0x7FFFFF
# @return 32-bit integer (the raw FP32 bit pattern)

sub encode_f32 {
    my ($sign, $exp, $mantissa) = @_;
    return (($sign & 0x1) << 31)
         | (($exp  & 0xFF) << 23)
         |  ($mantissa & 0x7FFFFF);
}

# ============================================================================
# decode_f32 — unpack a 32-bit integer into (sign, exp, mantissa)
# ============================================================================
#
# Inverse of encode_f32.  We extract each field using masks and right-shifts:
#
#   sign     = bits >> 31
#   exponent = (bits >> 23) & 0xFF
#   mantissa = bits & 0x7FFFFF
#
# @param $bits 32-bit integer
# @return ($sign, $exp, $mantissa) as a list

sub decode_f32 {
    my ($bits) = @_;
    my $sign     = ($bits >> 31) & 0x1;
    my $exp      = ($bits >> 23) & 0xFF;
    my $mantissa = $bits & 0x7FFFFF;
    return ($sign, $exp, $mantissa);
}

# ============================================================================
# _is_nan / _is_inf / _is_zero — detect special bit patterns
# ============================================================================
#
# These helpers detect IEEE 754 special values by checking the exponent and
# mantissa fields.  They operate on the raw 32-bit integer.

# NaN: exponent all-1s AND mantissa non-zero.
sub _is_nan  { my ($b) = @_; (($b >> 23) & 0xFF) == 0xFF && ($b & 0x7FFFFF) != 0 }
# Infinity: exponent all-1s AND mantissa zero.
sub _is_inf  { my ($b) = @_; (($b >> 23) & 0xFF) == 0xFF && ($b & 0x7FFFFF) == 0 }
# Zero: exponent all-0s AND mantissa zero.
sub _is_zero { my ($b) = @_; ($b & 0x7FFFFFFF) == 0 }

# Canonical bit patterns for special values.
use constant POS_INF => encode_f32(0, 0xFF, 0);
use constant NEG_INF => encode_f32(1, 0xFF, 0);
use constant POS_NAN => encode_f32(0, 0xFF, 0x400000);  # quiet NaN

# ============================================================================
# f32_add — add two FP32 bit-patterns
# ============================================================================
#
# === THE FIVE STEPS OF FP ADDITION ===
#
#   1. Handle special cases (NaN, Infinity, Zero).
#   2. Extract exponents and mantissas; restore the implicit leading 1.
#   3. Align mantissas: shift the operand with the SMALLER exponent right
#      by (exponent_difference) bits.
#   4. Add or subtract mantissas (depends on signs).
#   5. Normalize: adjust mantissa and exponent so the leading bit is in the
#      right position.
#   6. Round (round-to-nearest-even) and pack the result.
#
# === WORKED EXAMPLE: 1.5 + 0.25 ===
#
#   1.5  = sign=0, exp=127, stored_mant=0b1000_0000_0000_0000_0000_000
#           full_mant = 1.1000...0 = 0xC00000 (with implicit 1 at bit 23)
#
#   0.25 = sign=0, exp=125, stored_mant=0
#           full_mant = 1.0000...0 = 0x800000
#
#   exp_diff = 127 - 125 = 2 → shift 0.25's mantissa right by 2
#   aligned  = 0x200000
#
#   Add:   0xC00000 + 0x200000 = 0xE00000 = 1.1100...0  still normalized
#   Result: sign=0, exp=127, mant = 0xE00000 & 0x7FFFFF = 0x600000
#   That's 1.5 + 0.25 = 1.75 ✓
#
# @param $a  32-bit FP32 bit pattern
# @param $b  32-bit FP32 bit pattern
# @return    32-bit FP32 result

sub f32_add {
    my ($a, $b) = @_;

    # --- Step 1: Special cases ---
    return POS_NAN if _is_nan($a) || _is_nan($b);

    my $a_inf = _is_inf($a);
    my $b_inf = _is_inf($b);
    if ($a_inf && $b_inf) {
        # Inf + (-Inf) = NaN; same-sign infinities = Inf with that sign
        my $sa = ($a >> 31) & 1;
        my $sb = ($b >> 31) & 1;
        return ($sa == $sb) ? $a : POS_NAN;
    }
    return $a if $a_inf;
    return $b if $b_inf;

    return $b if _is_zero($a);
    return $a if _is_zero($b);

    # --- Step 2: Extract fields ---
    my ($sa, $ea, $ma) = decode_f32($a);
    my ($sb, $eb, $mb) = decode_f32($b);

    # Restore the implicit leading 1 for normal numbers.
    # For denormals (exponent == 0), the implicit bit is 0; we use exp=1
    # to represent their true exponent of (1 - 127 = -126).
    if ($ea != 0) { $ma |= (1 << 23); } else { $ea = 1; }
    if ($eb != 0) { $mb |= (1 << 23); } else { $eb = 1; }

    # Use extra guard bits for precision during rounding.
    # Guard (G), Round (R), Sticky (S) — 3 extra bits on the right.
    my $G = 3;
    $ma <<= $G;
    $mb <<= $G;

    # --- Step 3: Align mantissas ---
    my $result_exp;
    if ($ea >= $eb) {
        my $diff = $ea - $eb;
        if ($diff > 0) {
            my $sticky = ($mb & ((1 << $diff) - 1)) ? 1 : 0;
            $mb >>= $diff;
            $mb |= $sticky;
        }
        $result_exp = $ea;
    } else {
        my $diff = $eb - $ea;
        if ($diff > 0) {
            my $sticky = ($ma & ((1 << $diff) - 1)) ? 1 : 0;
            $ma >>= $diff;
            $ma |= $sticky;
        }
        $result_exp = $eb;
    }

    # --- Step 4: Add or subtract mantissas based on signs ---
    my ($result_mant, $result_sign);
    if ($sa == $sb) {
        $result_mant = $ma + $mb;
        $result_sign = $sa;
    } else {
        if ($ma >= $mb) {
            $result_mant = $ma - $mb;
            $result_sign = $sa;
        } else {
            $result_mant = $mb - $ma;
            $result_sign = $sb;
        }
    }

    return encode_f32(0, 0, 0) if $result_mant == 0;  # exact cancellation → +0

    # --- Step 5: Normalize ---
    # The normal position for the leading 1 is at bit (23 + G).
    my $normal_pos = 23 + $G;
    my $leading    = _bit_length($result_mant) - 1;

    if ($leading > $normal_pos) {
        my $shift = $leading - $normal_pos;
        my $sticky = ($result_mant & ((1 << $shift) - 1)) ? 1 : 0;
        $result_mant >>= $shift;
        $result_mant |= $sticky;
        $result_exp  += $shift;
    } elsif ($leading < $normal_pos) {
        my $shift = $normal_pos - $leading;
        if ($result_exp - $shift >= 1) {
            $result_mant <<= $shift;
            $result_exp  -= $shift;
        } else {
            # Becoming denormal
            my $actual = $result_exp - 1;
            $result_mant <<= $actual if $actual > 0;
            $result_exp = 0;
        }
    }

    # --- Step 6: Round to nearest even ---
    my $guard  = ($result_mant >> ($G - 1)) & 1;
    my $round  = ($result_mant >> ($G - 2)) & 1;
    my $sticky = ($result_mant & ((1 << ($G - 2)) - 1)) ? 1 : 0;
    $result_mant >>= $G;
    if ($guard) {
        if ($round || $sticky || ($result_mant & 1)) {
            $result_mant++;
            if ($result_mant >= (1 << 24)) {
                $result_mant >>= 1;
                $result_exp++;
            }
        }
    }

    # --- Step 7: Handle exponent overflow / underflow ---
    return encode_f32($result_sign, 0xFF, 0) if $result_exp >= 0xFF;
    if ($result_exp <= 0) {
        return encode_f32($result_sign, 0, 0) if $result_exp < -23;
        $result_mant >>= (1 - $result_exp);
        $result_exp = 0;
    }

    # Remove the implicit leading 1 for normal numbers
    $result_mant &= 0x7FFFFF if $result_exp > 0;

    return encode_f32($result_sign, $result_exp, $result_mant);
}

# ============================================================================
# f32_mul — multiply two FP32 bit-patterns
# ============================================================================
#
# === THE FOUR STEPS OF FP MULTIPLICATION ===
#
# Multiplication is simpler than addition because you don't need to align
# mantissas.  You can work directly in scientific notation:
#
#   (s1, e1, 1.m1)  *  (s2, e2, 1.m2)
#   = (s1 XOR s2,  e1 + e2 - bias,  1.m1 * 1.m2)
#
#   Step 1: result_sign = sign_a XOR sign_b
#   Step 2: result_exp  = exp_a + exp_b - BIAS
#   Step 3: result_mant = full_mant_a * full_mant_b  (48-bit product)
#   Step 4: Normalize and round
#
# === WORKED EXAMPLE: 1.5 * 2.0 ===
#
#   1.5 = exp=127, full_mant = 1.1_binary = 0xC00000
#   2.0 = exp=128, full_mant = 1.0_binary = 0x800000
#
#   result_exp = 127 + 128 - 127 = 128
#   product    = 0xC00000 * 0x800000 = 0x600000_000000
#   Normalize: leading bit at position 47; we want position 46 (2*23).
#              shift right 1 → increment exp to 129? No wait...
#
#   Actually: 0xC00000 = 12582912, 0x800000 = 8388608
#   product = 12582912 * 8388608 = 105553116266496
#   = 0x600000000000 (47 bits)
#   normal_pos = 2*23 = 46, leading = 46 → already normalized
#   round_pos = 46 - 23 = 23
#   result_mant = product >> 23 = 0xC00000
#   Remove implicit 1: 0xC00000 & 0x7FFFFF = 0x400000
#   result_exp = 128  → true_exp = 1 → 2^1 * 1.5 = 3.0 ✓
#
# @param $a  32-bit FP32 bit pattern
# @param $b  32-bit FP32 bit pattern
# @return    32-bit FP32 result

sub f32_mul {
    my ($a, $b) = @_;

    # Result sign: XOR of input signs (even for special cases)
    my $result_sign = (($a >> 31) ^ ($b >> 31)) & 1;

    # Special cases
    return POS_NAN if _is_nan($a) || _is_nan($b);

    my $a_inf  = _is_inf($a);
    my $b_inf  = _is_inf($b);
    my $a_zero = _is_zero($a);
    my $b_zero = _is_zero($b);

    # Inf * 0 = NaN (undefined)
    return POS_NAN if ($a_inf && $b_zero) || ($b_inf && $a_zero);
    return encode_f32($result_sign, 0xFF, 0) if $a_inf || $b_inf;
    return encode_f32($result_sign, 0, 0)    if $a_zero || $b_zero;

    # Extract fields
    my ($sa, $ea, $ma) = decode_f32($a);
    my ($sb, $eb, $mb) = decode_f32($b);

    # Restore implicit leading 1
    if ($ea != 0) { $ma |= (1 << 23); } else { $ea = 1; }
    if ($eb != 0) { $mb |= (1 << 23); } else { $eb = 1; }

    # Step 2: add exponents, subtract bias once
    my $result_exp = $ea + $eb - BIAS;

    # Step 3: multiply mantissas → up to 48-bit product
    my $product = $ma * $mb;

    # Step 4: normalize
    # For two 24-bit values, the product is at most 48 bits wide.
    # The leading 1 should be at position 2*23 = 46 (0-indexed) after
    # we remove the implicit leading 1, so the normal product leading bit
    # is at position 46 or 47.
    my $leading    = _bit_length($product) - 1;
    my $normal_pos = 2 * 23;  # = 46
    my $result_mant;

    my $round_pos = $leading - 23;

    if ($round_pos > 0) {
        my $guard  = ($product >> ($round_pos - 1)) & 1;
        my $round  = 0;
        my $sticky = 0;
        if ($round_pos >= 2) {
            $round  = ($product >> ($round_pos - 2)) & 1;
            $sticky = ($product & ((1 << ($round_pos - 2)) - 1)) ? 1 : 0;
        }

        $result_mant = $product >> $round_pos;

        # Round to nearest even
        if ($guard) {
            if ($round || $sticky || ($result_mant & 1)) {
                $result_mant++;
                if ($result_mant >= (1 << 24)) {
                    $result_mant >>= 1;
                    $result_exp++;
                }
            }
        }

        # Account for normalization shift in exponent
        $result_exp += ($leading - $normal_pos) if $leading > $normal_pos;
        $result_exp -= ($normal_pos - $leading) if $leading < $normal_pos;

    } elsif ($round_pos == 0) {
        $result_mant = $product;
    } else {
        $result_mant = $product << (-$round_pos);
    }

    # Handle exponent overflow/underflow
    return encode_f32($result_sign, 0xFF, 0) if $result_exp >= 0xFF;
    if ($result_exp <= 0) {
        return encode_f32($result_sign, 0, 0) if $result_exp < -23;
        $result_mant >>= (1 - $result_exp);
        $result_exp = 0;
    }

    # Remove the implicit leading 1
    $result_mant &= 0x7FFFFF if $result_exp > 0;

    return encode_f32($result_sign, $result_exp, $result_mant);
}

# ============================================================================
# f32_to_string — human-readable representation of an FP32 bit pattern
# ============================================================================
#
# Returns something like "0x3FC00000 (+, exp=127, mant=0x400000) = 1.5"
# or "NaN", "+Inf", "-Inf", "+0", "-0".
#
# @param $bits  32-bit FP32 bit pattern
# @return string

sub f32_to_string {
    my ($bits) = @_;
    return "NaN"  if _is_nan($bits);
    return "+Inf" if $bits == POS_INF;
    return "-Inf" if $bits == NEG_INF;

    my ($sign, $exp, $mant) = decode_f32($bits);

    if ($exp == 0 && $mant == 0) {
        return ($sign ? "-0" : "+0");
    }

    my $float_val = f32_to_float($bits);
    return sprintf("%s, exp=%d, mant=0x%06X) = %g",
        ($sign ? "-" : "+"), $exp, $mant, $float_val);
}

# ============================================================================
# float_to_f32 — convert a Perl native float to an FP32 bit pattern
# ============================================================================
#
# We use Perl's pack/unpack to get the hardware's FP32 representation,
# then re-read it as a 32-bit unsigned integer.  This is the same technique
# used in the Lua reference implementation.
#
# pack("f", $value) packs $value as a native C float (single-precision).
# unpack("N", ...) interprets the 4 bytes as a big-endian 32-bit unsigned int.
# unpack("V", ...) is little-endian; we must account for the host byte order.
# Using "L>" (big-endian unsigned long) is safest for cross-platform code.
#
# @param $float  Perl number
# @return 32-bit integer (FP32 bit pattern)

sub float_to_f32 {
    my ($float) = @_;
    my $bytes = pack("f", $float);
    # Unpack as native unsigned 32-bit — same byte order as pack("f",...)
    my ($bits) = unpack("L", $bytes);
    return $bits;
}

# ============================================================================
# f32_to_float — convert an FP32 bit pattern to a Perl native float
# ============================================================================
#
# Inverse of float_to_f32: pack the bits as an unsigned 32-bit int, then
# reinterpret as a C float.
#
# @param $bits 32-bit integer
# @return Perl native float

sub f32_to_float {
    my ($bits) = @_;
    my $bytes = pack("L", $bits);
    my ($float) = unpack("f", $bytes);
    return $float;
}

# ============================================================================
# _bit_length — number of bits needed to represent $n (position of highest set bit + 1)
# ============================================================================
#
# For example:
#   _bit_length(0) = 0
#   _bit_length(1) = 1
#   _bit_length(4) = 3   (binary: 100)
#   _bit_length(7) = 3   (binary: 111)
#
# We use a simple loop here for clarity, mirroring the Lua reference.
# A faster version would use log2, but clarity matters more here.

sub _bit_length {
    my ($n) = @_;
    return 0 if $n == 0;
    my $len = 0;
    my $v   = $n;
    while ($v > 0) { $v >>= 1; $len++; }
    return $len;
}

1;

__END__

=head1 NAME

CodingAdventures::FpArithmetic - IEEE 754 single-precision FP arithmetic

=head1 SYNOPSIS

    use CodingAdventures::FpArithmetic qw(
        encode_f32 decode_f32
        f32_add f32_mul
        f32_to_string float_to_f32 f32_to_float
    );

    my $a = float_to_f32(1.5);
    my $b = float_to_f32(0.25);
    my $c = f32_add($a, $b);
    print f32_to_string($c);   # 1.75

=head1 DESCRIPTION

Educational IEEE 754 single-precision floating-point arithmetic built from
scratch.  All core operations (encode, decode, add, mul) operate on raw 32-bit
integers and manipulate bit fields directly.

=head1 FUNCTIONS

=head2 encode_f32($sign, $exp, $mantissa)

Pack sign, biased exponent, and 23-bit mantissa into a 32-bit integer.

=head2 decode_f32($bits)

Unpack a 32-bit integer into (sign, biased_exponent, mantissa).

=head2 f32_add($a, $b)

Add two FP32 bit-patterns using the IEEE 754 addition algorithm.

=head2 f32_mul($a, $b)

Multiply two FP32 bit-patterns using the IEEE 754 multiplication algorithm.

=head2 f32_to_string($bits)

Return a human-readable string describing the FP32 value.

=head2 float_to_f32($float)

Convert a Perl native float to its FP32 bit pattern.

=head2 f32_to_float($bits)

Convert an FP32 bit pattern to a Perl native float.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
