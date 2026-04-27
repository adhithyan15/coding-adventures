package CodingAdventures::Intel8008GateLevel::Bits;

# ============================================================================
# Bits.pm — Bit Conversion and Parity Helpers
# ============================================================================
#
# This module provides the bit-level plumbing for the Intel 8008 gate-level
# simulator. All arithmetic and logic in the gate-level simulator operates
# on lists of bits rather than integers, mirroring how real digital circuits
# process binary signals wire by wire.
#
# ## LSB-First Convention
#
# All bit arrays in this package are LSB-first (least-significant bit at
# index 0), matching the convention used by CodingAdventures::Arithmetic:
#
#   int_to_bits(5, 8) → [1, 0, 1, 0, 0, 0, 0, 0]
#                           ^                   ^
#                         bit0 (LSB)           bit7 (MSB)
#
# This convention is natural for ripple-carry adders: bit0 is connected to
# the first full-adder, which produces bit0 of the sum and a carry to bit1.
#
# ## Parity via XOR Reduction
#
# The Intel 8008 parity flag (P) is defined as:
#   P=1 when the result has an EVEN number of 1-bits (even parity).
#   P=0 when the result has an ODD number of 1-bits (odd parity).
#
# Hardware parity is computed by a tree of XOR gates:
#   xor_chain = XOR(b0, XOR(b1, XOR(b2, ... XOR(b6, b7))))
#   P = NOT(xor_chain)
#
# XORn(all bits) = 1 when odd number of 1s → inverted = P = 0 (odd parity)
# XORn(all bits) = 0 when even number of 1s → inverted = P = 1 (even parity)
#
# The 8008 parity tree uses 7 XOR gates for 8 bits:
#   Level 1: XOR(b0,b1), XOR(b2,b3), XOR(b4,b5), XOR(b6,b7)   [4 XOR gates]
#   Level 2: XOR(r01,r23), XOR(r45,r67)                         [2 XOR gates]
#   Level 3: XOR(r0123, r4567)                                   [1 XOR gate]
#   Total: 7 gates for 3-level balanced tree (vs 7-gate linear chain)

use strict;
use warnings;

use CodingAdventures::LogicGates qw(NOT XORn);

use Exporter 'import';
our @EXPORT_OK = qw(int_to_bits bits_to_int compute_parity);

# int_to_bits — convert an integer to an LSB-first bit array.
#
# @param $value   Integer value to convert (masked to $width bits).
# @param $width   Number of bits in the result array.
# @return         Arrayref of 0/1 values, LSB at index 0.
#
# Example: int_to_bits(5, 8) → [1, 0, 1, 0, 0, 0, 0, 0]
#   5 = 0b00000101 in binary
#   bit0=1 (LSB), bit1=0, bit2=1, bit3=0, ..., bit7=0 (MSB)

sub int_to_bits {
    my ($value, $width) = @_;
    $value //= 0;
    $width //= 8;

    # Mask to $width bits to prevent negative values from spreading
    my $mask = ($width >= 32) ? 0xFFFFFFFF : ((1 << $width) - 1);
    $value = $value & $mask;

    my @bits;
    for my $i (0 .. $width - 1) {
        push @bits, ($value >> $i) & 1;
    }
    return \@bits;
}

# bits_to_int — convert an LSB-first bit array to an integer.
#
# @param $bits   Arrayref of 0/1 values, LSB at index 0.
# @return        Integer value.
#
# Example: bits_to_int([1, 0, 1, 0, 0, 0, 0, 0]) → 5

sub bits_to_int {
    my ($bits) = @_;
    my $value = 0;
    for my $i (0 .. $#$bits) {
        $value |= ($bits->[$i] << $i);
    }
    return $value;
}

# compute_parity — compute the 8008 parity flag from an 8-bit result.
#
# Uses XORn (N-input XOR chain) from the logic-gates package to reduce
# all 8 bits to a single parity bit, then inverts it to match the 8008
# convention (P=1 = even parity).
#
# Hardware path:
#   bits → XORn chain (7 XOR gates) → NOT gate → P flag
#
# @param @bits   8 individual bits (0 or 1 each), in any order.
# @return        1 if even parity (P=1, even count of 1-bits),
#                0 if odd parity (P=0, odd count of 1-bits).

sub compute_parity {
    my @bits = @_;
    # XORn returns 1 if an odd number of bits are 1.
    # The 8008 P flag = NOT(XORn): 1 means even parity.
    return NOT(XORn(@bits));
}

1;
