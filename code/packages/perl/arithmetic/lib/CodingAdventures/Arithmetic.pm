package CodingAdventures::Arithmetic;

# ============================================================================
# CodingAdventures::Arithmetic — Binary arithmetic circuits in Pure Perl
# ============================================================================
#
# # Moving from Logic to Math
#
# In the logic-gates package, we saw how transistors combine to form gates
# that perform basic Boolean operations (AND, OR, XOR, NOT). But how does a
# computer actually do *math*?
#
# This module answers that question by building arithmetic circuits from
# scratch, the same way real CPU designers do it. We start with the simplest
# possible adder (the Half Adder, which handles exactly one bit column) and
# work our way up to a complete ALU (Arithmetic Logic Unit) — the
# computational heart of every CPU.
#
# # The Building-Block Philosophy
#
# Notice that we never use Perl's '+' operator to add bits. Instead, we
# simulate the actual gate-level behaviour:
#
#   * XOR  computes the sum bit
#   * AND  computes the carry bit
#
# This is the same approach used in formal hardware description languages
# (VHDL, Verilog) — a technique called Register-Transfer Level (RTL) design.
#
# # Bit Representation
#
# Bits are plain integers: 0 or 1.
# Multi-bit numbers are Perl array references, stored **LSB-first** (Least
# Significant Bit at index 0). This matches the natural left-to-right
# processing order for carry propagation.
#
#   Example: the 4-bit number 5 (binary 0101) is stored as [1, 0, 1, 0]
#                                                             ^--- index 0 (LSB)
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# Export nothing by default; callers use fully-qualified names or OO API.

# ============================================================================
# Exported functions — pure combinational logic
# ============================================================================

# ----------------------------------------------------------------------------
# half_adder($a, $b) → ($sum, $carry)
#
# A half adder adds two single bits and produces two outputs:
#
#   * Sum   — the result in the current bit column
#   * Carry — the overflow into the next column
#
# Why "half"? Because it can GENERATE a carry but cannot ACCEPT one from a
# previous column. It has only two inputs (a, b), not three.
#
# Think of grade-school addition: 7 + 8 = 15. The "5" is the sum; the "1"
# is the carry to the tens column. In binary, 1 + 1 = 10₂ — "0" is the sum
# and "1" is the carry.
#
# Truth table:
#
#   A | B | Sum | Carry
#   --|---|-----|------
#   0 | 0 |  0  |   0
#   0 | 1 |  1  |   0
#   1 | 0 |  1  |   0
#   1 | 1 |  0  |   1    ← only case that generates a carry
#
# Pattern recognition:
#   * Sum   = XOR(A, B)  — "different bits give 1"
#   * Carry = AND(A, B)  — "both 1 gives a carry"
#
# Circuit diagram:
#
#   A ──┬──[XOR]── Sum
#       │
#   B ──┼──[AND]── Carry
#
# @param $a   First bit (0 or 1)
# @param $b   Second bit (0 or 1)
# @return ($sum, $carry)  Both are 0 or 1
# ----------------------------------------------------------------------------
sub half_adder {
    my ($a, $b) = @_;
    my $sum   = ($a ^ $b) & 1;   # XOR: differs → 1
    my $carry = ($a & $b) & 1;   # AND: both 1 → carry
    return ($sum, $carry);
}

# ----------------------------------------------------------------------------
# full_adder($a, $b, $cin) → ($sum, $cout)
#
# A full adder extends the half adder by accepting a THIRD input: a carry_in
# from a previous column. This is what makes multi-bit addition possible —
# every column beyond the first might receive a carry from the column to its
# right.
#
# How to build it? Chain two half adders together:
#
#   Step 1: half_adder(A, B) → partial_sum, partial_carry
#   Step 2: half_adder(partial_sum, Cin) → sum, carry2
#   Step 3: cout = OR(partial_carry, carry2)
#
# Why OR instead of XOR? The two half adders can never BOTH produce a carry
# of 1 simultaneously (proof: only when A=B=1 does the first half adder
# carry, which gives partial_sum=0; then partial_sum + Cin ≤ 1, so carry2=0).
# So OR and XOR are equivalent here — OR is the conventional choice.
#
# Truth table (8 rows because 3 inputs):
#
#   A | B | Cin | Sum | Cout
#   --|---|-----|-----|-----
#   0 | 0 |  0  |  0  |  0
#   0 | 0 |  1  |  1  |  0
#   0 | 1 |  0  |  1  |  0
#   0 | 1 |  1  |  0  |  1
#   1 | 0 |  0  |  1  |  0
#   1 | 0 |  1  |  0  |  1
#   1 | 1 |  0  |  0  |  1
#   1 | 1 |  1  |  1  |  1   ← 1+1+1=3=11₂, sum=1, carry=1
#
# @param $a    First bit (0 or 1)
# @param $b    Second bit (0 or 1)
# @param $cin  Carry-in from previous column (0 or 1)
# @return ($sum, $cout)
# ----------------------------------------------------------------------------
sub full_adder {
    my ($a, $b, $cin) = @_;
    my ($partial_sum, $partial_carry) = half_adder($a, $b);
    my ($sum, $carry2)               = half_adder($partial_sum, $cin);
    my $cout = ($partial_carry | $carry2) & 1;   # OR gate
    return ($sum, $cout);
}

# ----------------------------------------------------------------------------
# ripple_carry_adder(\@a_bits, \@b_bits, $cin) → (\@sum_bits, $cout)
#
# A ripple carry adder chains N full adders together to add two N-bit binary
# numbers. It works exactly like grade-school long addition: start from the
# rightmost (LSB) column and move left, passing each column's carry into the
# next.
#
# The name "ripple" comes from how the carry propagates: the carry from bit 0
# feeds into bit 1, whose carry feeds into bit 2, and so on. In the worst
# case (1111...1 + 0000...1), the carry must "ripple" through every single
# adder before the result is ready.
#
# Real modern CPUs use "Carry Lookahead Adders" that compute all carries in
# parallel. But the ripple carry adder is the simplest foundation that all
# faster designs build upon.
#
# Example: Adding 5 + 3 = 8 (4-bit numbers, LSB-first)
#
#   5 = 0101 → [1, 0, 1, 0]
#   3 = 0011 → [1, 1, 0, 0]
#
#   Column 0: FullAdder(1, 1, 0) → Sum=0, Carry=1
#   Column 1: FullAdder(0, 1, 1) → Sum=0, Carry=1
#   Column 2: FullAdder(1, 0, 1) → Sum=0, Carry=1
#   Column 3: FullAdder(0, 0, 1) → Sum=1, Carry=0
#
#   Result: [0, 0, 0, 1] = 1000₂ = 8 ✓
#
# @param \@a   Array ref of bits (0 or 1), LSB at index 0
# @param \@b   Array ref of bits (0 or 1), same length as \@a
# @param $cin  Initial carry-in (usually 0)
# @return (\@sum_bits, $cout)
# ----------------------------------------------------------------------------
sub ripple_carry_adder {
    my ($a_ref, $b_ref, $cin) = @_;
    my @a = @$a_ref;
    my @b = @$b_ref;

    die "ripple_carry_adder: a and b must have the same length"
        unless @a == @b;
    die "ripple_carry_adder: bit arrays must not be empty"
        unless @a > 0;

    my @sum_bits;
    my $carry = $cin;

    for my $i (0 .. $#a) {
        my ($bit_sum, $bit_carry) = full_adder($a[$i], $b[$i], $carry);
        push @sum_bits, $bit_sum;
        $carry = $bit_carry;
    }

    return (\@sum_bits, $carry);
}

# ============================================================================
# ALU — Arithmetic Logic Unit
# ============================================================================
#
# The ALU is the part of a CPU that executes arithmetic and logic commands.
# You give it two N-bit numbers (A and B) and an operation code. It routes
# those numbers through the appropriate circuit and outputs:
#
#   * value    — the N-bit result
#   * zero     — is the result all zeros? (used for equality: if A-B=0 → A=B)
#   * carry    — did unsigned addition overflow past the top bit?
#   * negative — is the MSB 1? (in two's complement, MSB=1 means negative)
#   * overflow — did signed arithmetic produce an impossible result?
#
# Supported operations (as string constants below):
#
#   ADD — binary addition via ripple carry adder
#   SUB — subtraction via two's complement: A - B = A + NOT(B) + 1
#   AND — bitwise AND (each bit independently)
#   OR  — bitwise OR (each bit independently)
#   XOR — bitwise XOR (each bit independently)
#   NOT — bitwise NOT of the A bus (B is ignored)
#   SHL — logical shift left by 1 (LSB becomes 0, MSB falls off)
#   SHR — logical shift right by 1 (MSB becomes 0, LSB falls off)
#
# ============================================================================

# Operation constants — these are the ALU's "instruction set"
use constant ADD => 'add';
use constant SUB => 'sub';
use constant AND => 'and';
use constant OR  => 'or';
use constant XOR => 'xor';
use constant NOT => 'not';
use constant SHL => 'shl';
use constant SHR => 'shr';

# ============================================================================
# Package CodingAdventures::Arithmetic::ALU
# ============================================================================

package CodingAdventures::Arithmetic::ALU;

use strict;
use warnings;

# ----------------------------------------------------------------------------
# new($bits) → ALU instance
#
# Creates a new ALU configured with the given bit width. All inputs must
# match this width; all outputs will have this width.
#
# Real CPUs are built with a fixed data bus width. An 8-bit CPU (like the
# 6502 in the Apple II) processes 8 bits at a time. A 64-bit CPU processes
# 64 bits at a time. Our ALU is parameterized for any width ≥ 1.
#
# @param $bits   Number of bits for all data buses (must be >= 1)
# @return blessed hashref
# ----------------------------------------------------------------------------
sub new {
    my ($class, $bits) = @_;
    die "ALU: bit width must be >= 1" unless defined $bits && $bits >= 1;
    return bless { bits => $bits }, $class;
}

# ----------------------------------------------------------------------------
# _twos_complement_negate(\@bits) → (\@negated_bits, $carry_out)
#
# Convert a binary number to its two's-complement negation.
#
# # Two's Complement: the clever trick behind negative numbers
#
# How do computers represent negative numbers using only 0s and 1s?
# They use two's complement. To negate a number x:
#
#   Step 1: Flip every bit (NOT operation).
#   Step 2: Add 1.
#
# Why does this work? For any x and its bitwise complement NOT(x):
#
#   x + NOT(x) = 1111...1  (all ones)
#
# Adding 1 more makes it roll over to zero (ignoring the final carry):
#
#   x + NOT(x) + 1 = 0000...0
#
# Rearranging: NOT(x) + 1 = −x
#
# The beauty: the ALU uses the EXACT SAME adder circuit for both addition
# and subtraction. A - B = A + (−B) = A + NOT(B) + 1. No special subtraction
# hardware needed!
#
# @param $bits_ref   Array ref of bits (LSB-first)
# @return (\@negated, $carry)
# ----------------------------------------------------------------------------
sub _twos_complement_negate {
    my ($bits_ref) = @_;
    my @bits = @$bits_ref;

    # Step 1: Flip every bit
    my @inverted = map { $_ ^ 1 } @bits;

    # Step 2: Add 1 (binary 1 has only its LSB set)
    my @one = map { 0 } @bits;
    $one[0] = 1;

    return CodingAdventures::Arithmetic::ripple_carry_adder(\@inverted, \@one, 0);
}

# ----------------------------------------------------------------------------
# execute($op, \@a, \@b) → hashref
#
# The main ALU operation dispatcher. It reads the operation code and routes
# the input buses A and B into the appropriate circuit.
#
# In real hardware, this routing is done with *multiplexers* — circuits that
# select one of several inputs based on a control signal. Here we use
# if/elsif chains for the same purpose (which map cleanly to RTL IF statements
# in VHDL/Verilog).
#
# After computing the result, it calculates the four condition flags:
#   * zero     — all result bits are 0
#   * carry    — the carry out from the adder circuit
#   * negative — the MSB (most significant bit) of the result
#   * overflow — did signed arithmetic produce an impossible result?
#
# Overflow detection (signed arithmetic):
#
#   Overflow occurs when adding two numbers with the SAME sign produces a
#   result with a DIFFERENT sign. This is mathematically impossible and
#   indicates we "ran out of bits" to represent the magnitude.
#
#   Examples (4-bit two's complement, range -8 to +7):
#     5 + 5 = 10, but 10 doesn't fit in 4 signed bits → overflow
#     (-5) + (-5) = -10, doesn't fit → overflow
#     3 + (-2) = 1, different signs → NEVER overflows
#
# @param $op     Operation string (one of ADD/SUB/AND/OR/XOR/NOT/SHL/SHR)
# @param $a_ref  Array ref of bits for A bus (length must equal $self->{bits})
# @param $b_ref  Array ref of bits for B bus (required for binary ops)
# @return hashref { value => \@bits, zero => bool, carry => bool,
#                   overflow => bool, negative => bool }
# ----------------------------------------------------------------------------
sub execute {
    my ($self, $op, $a_ref, $b_ref) = @_;
    my @a = @$a_ref;
    my $n = $self->{bits};

    die "ALU: a length (@{[scalar @a]}) must match bit_width ($n)"
        unless @a == $n;

    if ($op ne CodingAdventures::Arithmetic::NOT &&
        $op ne CodingAdventures::Arithmetic::SHL &&
        $op ne CodingAdventures::Arithmetic::SHR) {
        die "ALU: b length (@{[scalar @$b_ref]}) must match bit_width ($n)"
            unless defined $b_ref && @$b_ref == $n;
    }

    my @b = defined $b_ref ? @$b_ref : ();
    my @value;
    my $carry_bit = 0;

    # ----------------------------------------------------------------
    # Step 1: Compute the result based on the operation code
    # ----------------------------------------------------------------

    if ($op eq CodingAdventures::Arithmetic::ADD) {
        # Straight addition through the ripple carry adder.
        my ($sum_ref, $c) = CodingAdventures::Arithmetic::ripple_carry_adder(\@a, \@b, 0);
        @value     = @$sum_ref;
        $carry_bit = $c;

    } elsif ($op eq CodingAdventures::Arithmetic::SUB) {
        # Subtraction is addition in disguise: A - B = A + (-B).
        # We negate B using two's complement, then add normally.
        my ($neg_b_ref, $_dummy) = _twos_complement_negate(\@b);
        my ($sum_ref, $c) = CodingAdventures::Arithmetic::ripple_carry_adder(\@a, $neg_b_ref, 0);
        @value     = @$sum_ref;
        $carry_bit = $c;

    } elsif ($op eq CodingAdventures::Arithmetic::AND) {
        # Bitwise AND: the same AND gate replicated once per bit position.
        # All copies operate independently — no carry propagation.
        @value = map { $a[$_] & $b[$_] } 0 .. $#a;

    } elsif ($op eq CodingAdventures::Arithmetic::OR) {
        @value = map { $a[$_] | $b[$_] } 0 .. $#a;

    } elsif ($op eq CodingAdventures::Arithmetic::XOR) {
        @value = map { $a[$_] ^ $b[$_] } 0 .. $#a;

    } elsif ($op eq CodingAdventures::Arithmetic::NOT) {
        # NOT is unary: it only operates on the A bus. B is ignored.
        # Flip every bit with XOR-1 (same as bitwise complement masked to 1 bit).
        @value = map { $_ ^ 1 } @a;

    } elsif ($op eq CodingAdventures::Arithmetic::SHL) {
        # Logical shift left: insert 0 at LSB, drop the MSB (which becomes carry).
        # In hardware this is just a rewiring — bit i connects to position i+1.
        $carry_bit = $a[$n - 1];            # the MSB falls off as carry
        @value = (0, @a[0 .. $n - 2]);      # prepend 0, drop last

    } elsif ($op eq CodingAdventures::Arithmetic::SHR) {
        # Logical shift right: insert 0 at MSB, drop the LSB (which becomes carry).
        $carry_bit = $a[0];                 # the LSB falls off as carry
        @value = (@a[1 .. $n - 1], 0);     # drop first, append 0

    } else {
        die "ALU: unknown operation: $op";
    }

    # ----------------------------------------------------------------
    # Step 2: Compute the condition flags
    # ----------------------------------------------------------------

    # Zero flag: true when every result bit is 0.
    # This is the basis for equality checks: compute A - B, then check zero.
    my $zero = 1;
    for my $bit (@value) {
        if ($bit != 0) { $zero = 0; last; }
    }

    # Negative flag: the most significant bit (MSB = last element in LSB-first array).
    # In two's complement: MSB=1 → negative number, MSB=0 → non-negative.
    my $negative = $value[$n - 1] == 1 ? 1 : 0;

    # Carry flag: did unsigned addition overflow past the top bit?
    my $carry = $carry_bit ? 1 : 0;

    # Overflow flag: did signed arithmetic produce a sign change that
    # shouldn't be possible?  Only relevant for ADD and SUB.
    my $overflow = 0;
    if ($op eq CodingAdventures::Arithmetic::ADD || $op eq CodingAdventures::Arithmetic::SUB) {
        my $a_sign = $a[$n - 1];
        my $b_sign;
        if ($op eq CodingAdventures::Arithmetic::ADD) {
            $b_sign = $b[$n - 1];
        } else {
            # For SUB A-B we're really computing A + NOT(B) + 1.
            # The effective sign of the second operand is the *inverse* of B's MSB.
            $b_sign = $b[$n - 1] ^ 1;
        }
        my $result_sign = $value[$n - 1];

        # Overflow iff both operands had the same sign but the result differs.
        if ($a_sign == $b_sign && $result_sign != $a_sign) {
            $overflow = 1;
        }
    }

    return {
        value    => \@value,
        zero     => $zero     ? 1 : 0,
        carry    => $carry    ? 1 : 0,
        overflow => $overflow ? 1 : 0,
        negative => $negative ? 1 : 0,
    };
}

# ============================================================================
# Back to main package — re-export the constants via the main namespace
# ============================================================================

package CodingAdventures::Arithmetic;

# Re-export ALU operation constants from the top-level namespace
# so callers can write CodingAdventures::Arithmetic::ADD etc.
# (They are already defined via 'use constant' above.)

1;

__END__

=head1 NAME

CodingAdventures::Arithmetic - Binary arithmetic circuits in Pure Perl

=head1 SYNOPSIS

    use CodingAdventures::Arithmetic;

    # Half adder (two input bits)
    my ($sum, $carry) = CodingAdventures::Arithmetic::half_adder(1, 1);
    # sum=0, carry=1

    # Full adder (two bits + carry-in)
    my ($s, $c) = CodingAdventures::Arithmetic::full_adder(1, 1, 1);
    # s=1, c=1

    # Ripple carry adder (LSB-first bit arrays)
    my ($bits, $cout) = CodingAdventures::Arithmetic::ripple_carry_adder(
        [1,0,1,0],  # 5 in binary (LSB first)
        [1,1,0,0],  # 3 in binary (LSB first)
        0           # carry-in
    );
    # $bits = [0,0,0,1] (8), $cout = 0

    # ALU
    my $alu = CodingAdventures::Arithmetic::ALU->new(4);
    my $result = $alu->execute(
        CodingAdventures::Arithmetic::ADD,
        [1,0,1,0],   # 5
        [1,1,0,0],   # 3
    );
    # $result->{value} = [0,0,0,1] (8)
    # $result->{zero}  = 0
    # $result->{carry} = 0

=head1 DESCRIPTION

Implements binary arithmetic circuits from the ground up:

=over 4

=item * C<half_adder($a, $b)> — adds two single bits; returns (sum, carry)

=item * C<full_adder($a, $b, $cin)> — adds two bits plus carry-in; returns (sum, cout)

=item * C<ripple_carry_adder(\@a, \@b, $cin)> — chains N full adders for N-bit addition

=item * C<CodingAdventures::Arithmetic::ALU> — Arithmetic Logic Unit with 8 operations

=back

Bit arrays are LSB-first (index 0 = least significant bit).

=head1 ALU OPERATIONS

  ADD  SUB  AND  OR  XOR  NOT  SHL  SHR

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
