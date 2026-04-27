package CodingAdventures::Intel8008GateLevel::ALU;

# ============================================================================
# ALU.pm — 8-Bit Arithmetic Logic Unit (Gate-Level)
# ============================================================================
#
# The Intel 8008 ALU is 8 bits wide (double the 4004's 4-bit ALU). Every
# arithmetic operation routes through an 8-bit ripple-carry adder: 8 full-
# adders chained in series, where each adder's carry-out feeds the next
# adder's carry-in.
#
# ## 8-Bit Ripple-Carry Adder (8 full-adders = 40 AND/OR/XOR gates)
#
# Each full-adder uses:
#   sum   = XOR(XOR(a, b), cin)   [2 XOR gates]
#   cout  = OR(AND(a,b), AND(XOR(a,b), cin))  [1 AND, 1 XOR, 1 AND, 1 OR = 3 gates]
# Total: 5 gates per full-adder.
# 8 full-adders = 40 gates (vs 4004's 20 gates).
#
# ## Gate path for ADD B (opcode 0x80):
#
#   a_bits = int_to_bits(A, 8)     [8 wires from accumulator register]
#   b_bits = int_to_bits(B, 8)     [8 wires from register B]
#   (result_bits, carry_out) = ripple_carry_adder(a_bits, b_bits, cin=0)
#     → full_adder(a[0], b[0], 0)    → (sum[0], carry1)
#     → full_adder(a[1], b[1], carry1) → (sum[1], carry2)
#     ...
#     → full_adder(a[7], b[7], carry7) → (sum[7], carry_out)
#   result = bits_to_int(result_bits) & 0xFF
#
# ## Subtraction via Two's Complement
#
# SUB B computes A - B as A + (~B) + 1 (two's complement negation):
#   1. NOT each bit of B (8 NOT gates)
#   2. ripple_carry_adder(a_bits, ~b_bits, cin=1)
#   The carry-out from this sum indicates whether a borrow occurred.
#   On the 8008: CY=1 after SUB means the result REQUIRED a borrow
#   (unsigned A < B). This is the opposite convention from ARM.
#
# ## Bitwise Operations
#
# AND: 8 AND gates (one per bit pair)
# OR:  8 OR gates
# XOR: 8 XOR gates
#
# ## Parity
#
# The 8008 computes parity via a 7-gate XOR reduction tree (see Bits.pm).
# Parity is recomputed after every ALU operation.

use strict;
use warnings;

use CodingAdventures::LogicGates qw(AND OR NOT XOR XORn);
use CodingAdventures::Arithmetic;
use CodingAdventures::Intel8008GateLevel::Bits qw(int_to_bits bits_to_int compute_parity);

use Exporter 'import';
our @EXPORT_OK = qw(
    alu_add alu_sub alu_and alu_or alu_xor
    alu_inr alu_dcr
    alu_rlc alu_rrc alu_ral alu_rar
    compute_flags
);

# Delegate to arithmetic package
sub _ripple_carry_adder { CodingAdventures::Arithmetic::ripple_carry_adder(@_) }

# alu_add — 8-bit addition with carry.
#
# Passes both operands through the 8-bit ripple-carry adder.
# Returns: ($result_int, $carry_out, $flags_hashref)
#
# @param $a         8-bit integer (accumulator)
# @param $b         8-bit integer (source operand)
# @param $carry_in  0 or 1 (carry-in; 1 for ADC, 0 for ADD)
sub alu_add {
    my ($a, $b, $carry_in) = @_;
    $carry_in //= 0;
    my $a_bits = int_to_bits($a, 8);
    my $b_bits = int_to_bits($b, 8);
    my ($result_bits, $carry_out) = _ripple_carry_adder($a_bits, $b_bits, $carry_in);
    my $result = bits_to_int($result_bits) & 0xFF;
    return ($result, $carry_out, compute_flags($result, $carry_out));
}

# alu_sub — 8-bit subtraction with borrow.
#
# Two's complement: a - b - borrow_in = a + (~b) + (1 - borrow_in)
# On the 8008, CY=1 after SUB means a borrow occurred (unsigned a < b).
# The carry convention: carry_in=1-borrow_in (no borrow = cin=1 = a + ~b + 1 = a-b).
#
# @param $a         8-bit integer (accumulator)
# @param $b         8-bit integer (source operand)
# @param $borrow_in 0 or 1 (carry-in = previous CY for SBB; 0 for SUB)
sub alu_sub {
    my ($a, $b, $borrow_in) = @_;
    $borrow_in //= 0;
    my $a_bits = int_to_bits($a, 8);
    my $b_bits = int_to_bits($b, 8);

    # Two's complement: NOT each bit of B (8 NOT gates)
    my $not_b_bits = [map { NOT($_) } @$b_bits];

    # Add A + ~B + (1 - borrow_in). cin=1 means no borrow (A - B = A + ~B + 1).
    my $cin = $borrow_in ? 0 : 1;
    my ($result_bits, $carry_out) = _ripple_carry_adder($a_bits, $not_b_bits, $cin);
    my $result = bits_to_int($result_bits) & 0xFF;

    # On the 8008: carry_out=0 from this computation means a borrow occurred.
    # CY = NOT(carry_out) when borrow_in=0.
    # For SBB with borrow_in=1: CY = NOT(carry_out) still.
    my $cy = $carry_out ? 0 : 1;  # 8008 borrow convention
    return ($result, $cy, compute_flags($result, $cy));
}

# alu_and — 8-bit AND. Clears carry.
sub alu_and {
    my ($a, $b) = @_;
    my $a_bits = int_to_bits($a, 8);
    my $b_bits = int_to_bits($b, 8);
    my @result_bits = map { AND($a_bits->[$_], $b_bits->[$_]) } 0..7;
    my $result = bits_to_int(\@result_bits) & 0xFF;
    return ($result, 0, compute_flags($result, 0));  # AND always clears carry
}

# alu_or — 8-bit OR. Clears carry.
sub alu_or {
    my ($a, $b) = @_;
    my $a_bits = int_to_bits($a, 8);
    my $b_bits = int_to_bits($b, 8);
    my @result_bits = map { OR($a_bits->[$_], $b_bits->[$_]) } 0..7;
    my $result = bits_to_int(\@result_bits) & 0xFF;
    return ($result, 0, compute_flags($result, 0));  # OR always clears carry
}

# alu_xor — 8-bit XOR. Clears carry.
sub alu_xor {
    my ($a, $b) = @_;
    my $a_bits = int_to_bits($a, 8);
    my $b_bits = int_to_bits($b, 8);
    my @result_bits = map { XOR($a_bits->[$_], $b_bits->[$_]) } 0..7;
    my $result = bits_to_int(\@result_bits) & 0xFF;
    return ($result, 0, compute_flags($result, 0));  # XOR always clears carry
}

# alu_inr — increment by 1. Preserves carry (carry-neutral).
# Uses ripple_carry_adder with B=1 and carry_in=0.
sub alu_inr {
    my ($a, $old_carry) = @_;
    my $a_bits = int_to_bits($a, 8);
    my $one_bits = int_to_bits(1, 8);
    my ($result_bits, undef) = _ripple_carry_adder($a_bits, $one_bits, 0);
    my $result = bits_to_int($result_bits) & 0xFF;
    # INR/DCR do NOT update CY — return old_carry unchanged
    return ($result, compute_flags_no_carry($result, $old_carry));
}

# alu_dcr — decrement by 1. Preserves carry.
# Uses ripple_carry_adder with B=0xFF (two's complement -1) and carry_in=0.
# a - 1 = a + 0xFF (wraps naturally in 8-bit arithmetic).
sub alu_dcr {
    my ($a, $old_carry) = @_;
    my $a_bits   = int_to_bits($a,    8);
    my $ff_bits  = int_to_bits(0xFF,  8);
    my ($result_bits, undef) = _ripple_carry_adder($a_bits, $ff_bits, 0);
    my $result = bits_to_int($result_bits) & 0xFF;
    # INR/DCR do NOT update CY
    return ($result, compute_flags_no_carry($result, $old_carry));
}

# alu_rlc — rotate left circular.
# CY ← A[7]; A ← (A << 1) | A[7]
# No gate array needed — just wire bit7 to both CY and bit0.
sub alu_rlc {
    my ($a) = @_;
    my $bit7 = ($a >> 7) & 1;
    my $result = (($a << 1) | $bit7) & 0xFF;
    return ($result, $bit7);  # (new_A, new_CY)
}

# alu_rrc — rotate right circular.
# CY ← A[0]; A ← (A >> 1) | (A[0] << 7)
sub alu_rrc {
    my ($a) = @_;
    my $bit0 = $a & 1;
    my $result = (($a >> 1) | ($bit0 << 7)) & 0xFF;
    return ($result, $bit0);  # (new_A, new_CY)
}

# alu_ral — rotate left through carry (9-bit rotation).
# new_A[0] ← old_CY; new_CY ← old_A[7]
sub alu_ral {
    my ($a, $carry_in) = @_;
    my $bit7   = ($a >> 7) & 1;
    my $result = (($a << 1) | $carry_in) & 0xFF;
    return ($result, $bit7);  # (new_A, new_CY)
}

# alu_rar — rotate right through carry (9-bit rotation).
# new_A[7] ← old_CY; new_CY ← old_A[0]
sub alu_rar {
    my ($a, $carry_in) = @_;
    my $bit0   = $a & 1;
    my $result = (($a >> 1) | ($carry_in << 7)) & 0xFF;
    return ($result, $bit0);  # (new_A, new_CY)
}

# compute_flags — compute all 4 flags from an 8-bit result and carry.
#
# Gate paths:
#   zero   = NOR8(b7, b6, b5, b4, b3, b2, b1, b0) [8-input NOR = NOT(OR8)]
#   sign   = b7  [direct wire]
#   carry  = carry_out from adder  [direct wire]
#   parity = NOT(XORn(b7,...,b0))  [7 XOR gates + NOT]
#
# @param $result   8-bit integer result
# @param $carry    carry/borrow bit (0 or 1)
# @return          hashref {carry, zero, sign, parity}

sub compute_flags {
    my ($result, $carry) = @_;
    my $r8   = $result & 0xFF;
    my $bits = int_to_bits($r8, 8);
    return {
        carry  => $carry ? 1 : 0,
        zero   => ($r8 == 0) ? 1 : 0,
        sign   => $bits->[7],          # bit7 = sign bit (MSB)
        parity => compute_parity(@$bits),
    };
}

# compute_flags_no_carry — like compute_flags but preserves existing carry.
# Used by INR and DCR instructions which must not update CY.
sub compute_flags_no_carry {
    my ($result, $old_carry) = @_;
    my $flags = compute_flags($result, $old_carry);
    $flags->{carry} = $old_carry ? 1 : 0;  # restore old carry
    return $flags;
}

1;
