package CodingAdventures::LogicGates;

# ============================================================================
# CodingAdventures::LogicGates — Combinational and Sequential Logic in Perl
# ============================================================================
#
# This module is part of the coding-adventures project, an educational
# computing stack built from transistors up through interpreters and compilers.
# It sits at layer 20 in the stack — directly above transistors.
#
# ## Two Kinds of Logic
#
# COMBINATIONAL logic: outputs depend ONLY on current inputs.  No memory,
# no history, no state.  Like a calculator: press "2 + 3", always get "5".
#
# SEQUENTIAL logic: outputs depend on current inputs AND previous state.
# Sequential circuits CAN REMEMBER.  This is the fundamental difference
# between a calculator (combinational) and a computer (sequential).
#
# This module implements both families.
#
# ## The Seven Fundamental Gates
#
#   Gate  | Symbol | Truth Table (A,B -> Out)
#   ------|--------|-------------------------
#   AND   |  A*B   | 0,0->0  0,1->0  1,0->0  1,1->1
#   OR    |  A+B   | 0,0->0  0,1->1  1,0->1  1,1->1
#   NOT   |  ~A    | 0->1  1->0  (unary)
#   XOR   |  A^B   | 0,0->0  0,1->1  1,0->1  1,1->0
#   NAND  | ~(A*B) | 0,0->1  0,1->1  1,0->1  1,1->0
#   NOR   | ~(A+B) | 0,0->1  0,1->0  1,0->0  1,1->0
#   XNOR  | ~(A^B) | 0,0->1  0,1->0  1,0->0  1,1->1
#
# ## NAND as the Universal Gate
#
# NAND is called a "universal gate" because ANY other Boolean function can be
# built from NANDs alone.  Real chip fabrication processes (CMOS) often build
# everything from NAND because the transistor layout is most compact.
#
#   NOT from NAND:   NAND(A, A)
#   AND from NAND:   NOT(NAND(A,B))
#   OR  from NAND:   NAND(NAND(A,A), NAND(B,B))
#   XOR from NAND:   Let C = NAND(A,B); NAND(NAND(A,C), NAND(B,C))
#
# Usage:
#
#   use CodingAdventures::LogicGates qw(AND OR NOT XOR NAND NOR XNOR
#                                        NAND_NOT NAND_AND NAND_OR NAND_XOR
#                                        ANDn ORn
#                                        SRLatch DLatch DFlipFlop
#                                        Register ShiftRegister Counter
#                                        new_flip_flop_state new_counter_state);
#
# All functions are plain functions (no class-method call style; see _args).
#
# ============================================================================

use strict;
use warnings;
use Scalar::Util ();

our $VERSION = '0.6.0';

use CodingAdventures::LogicGates::Error;

# Every rejection throws a typed CodingAdventures::LogicGates::Error (stable
# code + parameter name), never a bare string: see that file for why.
sub _throw {
    my ( $code, $param, $desc, $msg ) = @_;
    die CodingAdventures::LogicGates::Error->new(
        code => $code, param => $param, value_description => $desc, message => $msg );
}

# Public alias: true only for an exact-class INVALID_BIT error object.
sub is_invalid_bit { CodingAdventures::LogicGates::Error::is_invalid_bit(@_) }

# Import CMOS digital evaluation functions from the transistors package.
# Each of the seven primitive gates delegates its evaluation here, routing
# computation through a transistor physics simulation rather than using
# Perl's native bitwise operators.
#
#   NOT  → cmos_not(a)       (2 transistors: CMOS inverter)
#   NAND → cmos_nand(a, b)   (4 transistors: natural CMOS primitive)
#   NOR  → cmos_nor(a, b)    (4 transistors: natural CMOS primitive)
#   AND  → cmos_and(a, b)    (6 transistors: NAND + inverter)
#   OR   → cmos_or(a, b)     (6 transistors: NOR + inverter)
#   XOR  → cmos_xor(a, b)    (16 transistors: 4 NAND gates)
#   XNOR → cmos_xnor(a, b)   (18 transistors: XOR + inverter)
use CodingAdventures::Transistors qw(cmos_not cmos_nand cmos_nor cmos_and cmos_or cmos_xor cmos_xnor);

use Exporter 'import';
our @EXPORT_OK = qw(
    is_invalid_bit
    AND OR NOT XOR NAND NOR XNOR
    NAND_NOT NAND_AND NAND_OR NAND_XOR
    ANDn ORn XORn
    SRLatch DLatch DFlipFlop
    Register ShiftRegister Counter
    new_flip_flop_state new_counter_state
);

# ============================================================================
# Internal: _is_bit / _bit / _args
# ============================================================================
#
# In digital electronics a "bit" is a signal that is either LOW (0) or HIGH (1).
# Anything else is undefined behaviour in hardware -- a voltage outside the
# valid logic thresholds.  We enforce validity at run time with die().
#
# ## Why the obvious check is WRONG in Perl
#
# The obvious check is   die ... if $v != 0 && $v != 1;
# and it is a security bug (CWE-20 / CWE-843, type confusion), because the
# numeric operator != does not compare the VALUE it was given; it first
# NUMIFIES it:
#
#   "1abc" -> 1     (leading digits are parsed, the rest is ignored; the only
#                    consequence is a "isn't numeric" WARNING, not an error)
#   "a"    -> 0     "" -> 0     undef -> 0   (all silently become 0)
#   "0 but true" -> 0    (a special string Perl exempts from the warning)
#   " 1", "1.0", "01", "+1", "1\n" -> 1   (all numify to exactly 1)
#   a dualvar (Scalar::Util::dualvar(1, "evil")) -> 1 in numeric context
#   an object with an overloaded 0+ or == can answer whatever it likes
#
# So AND("1abc", 1) used to "work" and return 1, and AND(undef, 1) returned 0:
# garbage in, plausible-looking bit out, no error.  A validator must check the
# TYPE (is this a plain defined scalar spelled like a bit?) BEFORE it looks at
# the value, and it must do so on the STRING form, which for a dualvar is the
# part a human (or a log line, or a serializer) will actually see.
#
# ## The accepted spellings
#
# A value is a bit if and only if ALL of these hold:
#
#   1. it is defined                        (undef is rejected)
#   2. it is not a reference                (refs, blessed and overloaded
#                                            objects are rejected; we never
#                                            call their overloaded methods)
#   3. its string form matches  /\A[01]\z/ (exactly one character, 0 or 1;
#                                            \z, not $, so "1\n" is rejected)
#
# That accepts the integers 0 and 1 and the strings "0" and "1".  It also
# accepts the floating-point values 0.0 and 1.0 *because Perl stringifies them
# as "0" and "1"* -- by the time they reach us they are indistinguishable from
# the integers, and every arithmetic result such as 3-2 is one of these.  It
# rejects "1.0", " 1", "1\n", "01", "+1", "0 but true", "1abc", "", "-0",
# true/false-looking strings such as "true", undef and every reference.
#
# The check relies on no warnings: it never uses a numeric operator on an
# unvalidated value, so it behaves identically under -w, `no warnings`, and
# `use warnings FATAL => 'all'`.

sub _is_bit {
    my ($value) = @_;
    return 0 if !defined $value;
    return 0 if ref $value;
    return $value =~ /\A[01]\z/ ? 1 : 0;
}

# _describe -- render a rejected value for an error message without ever
# triggering overloading, undef warnings, or dumping megabytes of input.
sub _describe {
    my ($value) = @_;
    return 'undef' if !defined $value;
    return 'a ' . Scalar::Util::reftype($value) . ' reference' if ref $value;
    # Cut BEFORE copying: substr/length on the original scalar never build a
    # full copy, so a 10 MB hostile string costs us 32 characters, not 10 MB.
    my $long = length($value) > 32;
    my $text = substr( $value, 0, 32 );
    $text .= '...' if $long;
    $text =~ s/([^\x20-\x7e])/sprintf( '\\x%02x', ord $1 )/ge;
    return "'$text'";
}

# @param  $value   The value to validate.
# @param  $name    The parameter name, for the error message.

sub _bit {
    my ( $value, $name ) = @_;
    unless ( _is_bit($value) ) {
        my $d = _describe($value);
        _throw( 'INVALID_BIT', $name, $d, "logic_gates: $name must be 0 or 1, got $d\n" );
    }
    # CANONICALISE.  We have read the STRING form; from here on we compute
    # only on a fresh integer of our own.  This matters for dualvars: the
    # value dualvar(5, "1") passes the string test above but its NUMERIC part
    # is 5, and the transistor layer computes numerically.  Returning a fresh
    # IV (never the caller's scalar) makes the numeric part irrelevant, so
    # dualvar(5, "1") is simply the bit 1 -- the same answer the string gives.
    return $value eq '1' ? 1 : 0;
}

# _args -- unpack and arity-check a public function's arguments.
#
# Public functions are PLAIN FUNCTIONS: AND(1, 0).  They are not methods and
# never guess an invocant.  The old code guessed "method call" from the
# argument COUNT (and later from a leading package name), so AND(x, y, z)
# silently dropped x, and ANDn($pkg, 1, 0) silently dropped an input: every
# guess is a way for an argument to vanish without an error.  A leading
# argument that happens to equal the package name is therefore just data: it
# fails bit validation, or the arity check, like any other non-bit.
#
# @param  $fname   Function name for error messages.
# @param  $min     Minimum number of real arguments.
# @param  $max     Maximum number of real arguments.
# @param  $args    The caller's @_ as an arrayref.
# @return          The real arguments.

sub _args {
    my ( $fname, $min, $max, $args ) = @_;
    my @a = @{$args};
    if ( @a < $min || @a > $max ) {
        my $want = $min == $max ? $min : "$min to $max";
        _throw( 'INVALID_ARITY', undef, undef,
            "logic_gates: $fname takes $want argument(s), got " . scalar(@a) . "\n" );
    }
    return @a;
}

# ============================================================================
# The Seven Fundamental Gates
# ============================================================================

# AND — returns 1 only when BOTH inputs are 1.
#
# Circuit concept: two transistors in SERIES.
# Both must conduct for current to flow from Vcc to output.
#
# Truth table:
#
#   A | B | A AND B
#   --|---|--------
#   0 | 0 |   0
#   0 | 1 |   0
#   1 | 0 |   0
#   1 | 1 |   1
#
# In digital hardware this gate is the simplest form of a logical product.
# Its name comes from set theory: "A AND B are both true".
#
# @param  $a, $b   Inputs (0 or 1).
# @return          AND of a and b.

sub AND {
    my ( $a, $b ) = _args( 'AND', 2, 2, \@_ );
    $a = _bit($a, 'a');
    $b = _bit($b, 'b');
    # Delegate to the CMOS AND gate (NAND + inverter = 6 transistors).
    return cmos_and( $a, $b );
}

# OR — returns 1 when AT LEAST ONE input is 1.
#
# Circuit concept: two transistors in PARALLEL.
# Either transistor conducting pulls output high.
#
# Truth table:
#
#   A | B | A OR B
#   --|---|-------
#   0 | 0 |   0
#   0 | 1 |   1
#   1 | 0 |   1
#   1 | 1 |   1
#
# @param  $a, $b   Inputs (0 or 1).
# @return          OR of a and b.

sub OR {
    my ( $a, $b ) = _args( 'OR', 2, 2, \@_ );
    $a = _bit($a, 'a');
    $b = _bit($b, 'b');
    # Delegate to the CMOS OR gate (NOR + inverter = 6 transistors).
    return cmos_or( $a, $b );
}

# NOT — inverts its input: 0 becomes 1, 1 becomes 0.
#
# The simplest possible gate — just one transistor (an inverter).
# When A = 1, the transistor conducts, pulling output to GND (0).
# When A = 0, the transistor is off, output floats to Vcc (1).
#
# Truth table:
#
#   A | NOT A
#   --|------
#   0 |   1
#   1 |   0
#
# @param  $a   Input (0 or 1).
# @return      NOT of a.

sub NOT {
    my ($a) = _args( 'NOT', 1, 1, \@_ );
    $a = _bit($a, 'a');
    # Delegate to the CMOS inverter (2 transistors: 1 PMOS + 1 NMOS).
    return cmos_not( $a );
}

# XOR (exclusive OR) — returns 1 when inputs DIFFER.
#
# XOR answers: "Are these two bits DIFFERENT?"
# This makes it invaluable for:
#   - Comparison circuits (are two values equal?)
#   - Parity checking (is the number of 1s odd or even?)
#   - Arithmetic (the core of binary addition — XOR is the sum bit,
#     AND is the carry bit)
#
# Truth table:
#
#   A | B | A XOR B
#   --|---|--------
#   0 | 0 |   0      (same — false)
#   0 | 1 |   1      (different — true)
#   1 | 0 |   1      (different — true)
#   1 | 1 |   0      (same — false)
#
# @param  $a, $b   Inputs (0 or 1).
# @return          XOR of a and b.

sub XOR {
    my ( $a, $b ) = _args( 'XOR', 2, 2, \@_ );
    $a = _bit($a, 'a');
    $b = _bit($b, 'b');
    # Delegate to cmos_xor — the CMOS 4-NAND XOR construction (16 transistors).
    return cmos_xor( $a, $b );
}

# NAND — returns 0 only when BOTH inputs are 1.
#
# NAND = NOT(AND).  It is the "universal gate" — you can build ANY other
# Boolean function using only NAND gates.  This is not just theoretical:
# early chip designs (like the 7400 TTL series) implemented all logic
# using NAND as the primitive building block.
#
# Truth table:
#
#   A | B | A NAND B
#   --|---|--------
#   0 | 0 |   1
#   0 | 1 |   1
#   1 | 0 |   1
#   1 | 1 |   0
#
# @param  $a, $b   Inputs (0 or 1).
# @return          NAND of a and b.

sub NAND {
    my ( $a, $b ) = _args( 'NAND', 2, 2, \@_ );
    $a = _bit($a, 'a');
    $b = _bit($b, 'b');
    # Delegate to the CMOS NAND gate (4 transistors — the natural CMOS primitive).
    return cmos_nand( $a, $b );
}

# NOR — returns 1 only when BOTH inputs are 0.
#
# NOR = NOT(OR).  Like NAND, NOR is also a universal gate.
# Historical note: the Apollo Guidance Computer (1969) used about 5,600
# NOR gates as its ONLY logic element — every computation performed by
# the computer that landed humans on the moon was built from NOR gates.
#
# Truth table:
#
#   A | B | A NOR B
#   --|---|--------
#   0 | 0 |   1
#   0 | 1 |   0
#   1 | 0 |   0
#   1 | 1 |   0
#
# @param  $a, $b   Inputs (0 or 1).
# @return          NOR of a and b.

sub NOR {
    my ( $a, $b ) = _args( 'NOR', 2, 2, \@_ );
    $a = _bit($a, 'a');
    $b = _bit($b, 'b');
    # Delegate to the CMOS NOR gate (4 transistors — the other natural CMOS primitive).
    return cmos_nor( $a, $b );
}

# XNOR (exclusive NOR) — returns 1 when inputs are the SAME.
#
# XNOR = NOT(XOR).  It is the "equivalence" gate:
# "Are these two bits EQUAL?"
#
# Truth table:
#
#   A | B | A XNOR B
#   --|---|--------
#   0 | 0 |   1      (same — equal)
#   0 | 1 |   0      (different — not equal)
#   1 | 0 |   0      (different — not equal)
#   1 | 1 |   1      (same — equal)
#
# @param  $a, $b   Inputs (0 or 1).
# @return          XNOR of a and b.

sub XNOR {
    my ( $a, $b ) = _args( 'XNOR', 2, 2, \@_ );
    $a = _bit($a, 'a');
    $b = _bit($b, 'b');
    # Delegate to cmos_xnor — XOR followed by an inverter (18 transistors).
    return cmos_xnor( $a, $b );
}

# ============================================================================
# NAND-Derived Gates (Proving Functional Completeness)
# ============================================================================
#
# The following functions rebuild each fundamental gate using ONLY NAND.
# This demonstrates that NAND is "functionally complete" — it alone can
# express ANY Boolean function.

# NAND_NOT — NOT using only NAND gates.
#
# Trick: feed the same input to both sides of a NAND.
#
#   NAND(A, A) = NOT(A AND A) = NOT(A)
#
# Gate count: 1 NAND.
#
# @param  $a   Input (0 or 1).
# @return      NOT(a), computed using only NAND.

sub NAND_NOT {
    my ($a) = _args( 'NAND_NOT', 1, 1, \@_ );
    $a = _bit($a, 'a');
    return NAND( $a, $a );
}

# NAND_AND — AND using only NAND gates.
#
#   AND = NOT(NAND)  =>  NAND_AND(A,B) = NAND_NOT(NAND(A,B))
#
# Gate count: 2 NANDs.
#
# @param  $a, $b   Inputs (0 or 1).
# @return          A AND B, using only NAND.

sub NAND_AND {
    my ( $a, $b ) = _args( 'NAND_AND', 2, 2, \@_ );
    $a = _bit($a, 'a');
    $b = _bit($b, 'b');
    return NAND_NOT( NAND( $a, $b ) );
}

# NAND_OR — OR using only NAND gates.
#
# By De Morgan's theorem: A OR B = NOT(NOT A AND NOT B)
#                                 = NAND(NOT A, NOT B)
#                                 = NAND(NAND(A,A), NAND(B,B))
#
# Gate count: 3 NANDs.
#
# @param  $a, $b   Inputs (0 or 1).
# @return          A OR B, using only NAND.

sub NAND_OR {
    my ( $a, $b ) = _args( 'NAND_OR', 2, 2, \@_ );
    $a = _bit($a, 'a');
    $b = _bit($b, 'b');
    return NAND( NAND( $a, $a ), NAND( $b, $b ) );
}

# NAND_XOR — XOR using only NAND gates.
#
# The minimum-NAND construction for XOR uses 4 gates:
#
#   Let C = NAND(A, B)
#   XOR(A,B) = NAND(NAND(A, C), NAND(B, C))
#
# Verification:
#   A=0, B=0: C=1, NAND(0,1)=1, NAND(0,1)=1, NAND(1,1)=0  ✓
#   A=0, B=1: C=1, NAND(0,1)=1, NAND(1,1)=0, NAND(1,0)=1  ✓
#   A=1, B=0: C=1, NAND(1,1)=0, NAND(0,1)=1, NAND(0,1)=1  ✓
#   A=1, B=1: C=0, NAND(1,0)=1, NAND(1,0)=1, NAND(1,1)=0  ✓
#
# Gate count: 4 NANDs (minimum possible for XOR).
#
# @param  $a, $b   Inputs (0 or 1).
# @return          A XOR B, using only NAND.

sub NAND_XOR {
    my ( $a, $b ) = _args( 'NAND_XOR', 2, 2, \@_ );
    $a = _bit($a, 'a');
    $b = _bit($b, 'b');
    my $c = NAND( $a, $b );
    return NAND( NAND( $a, $c ), NAND( $b, $c ) );
}

# ============================================================================
# Multi-Input Gates
# ============================================================================
#
# Real circuits often need to AND or OR more than two signals together.
# Multi-input gates are built by chaining two-input gates (left-fold):
#
#   AND(A, B, C, D) = AND(AND(AND(A, B), C), D)

# ANDn — variadic AND, returns 1 only when ALL inputs are 1.
#
# @param  @inputs   Two or more inputs (each 0 or 1).
# @return           1 if all inputs are 1, else 0.

# _variadic_inputs -- accept (@bits) or a single exact ARRAY reference (\@bits).
# Nothing else is unwrapped or dropped; the individual elements are validated (and canonicalised) by the caller with _bit.
sub _variadic_inputs {
    my @a = @_;
    return @{ $a[0] } if @a == 1 && ref $a[0] eq 'ARRAY';
    return @a;
}

sub ANDn {
    my @inputs = _variadic_inputs(@_);

    _throw( 'INVALID_ARITY', 'input', undef, "logic_gates: ANDn requires at least 2 inputs\n" )
        if @inputs < 2;
    $_ = _bit( $_, "input" ) for @inputs;

    my $result = $inputs[0];
    for my $i ( 1 .. $#inputs ) {
        $result = AND( $result, $inputs[$i] );
    }
    return $result;
}

# ORn — variadic OR, returns 1 when AT LEAST ONE input is 1.
#
# @param  @inputs   Two or more inputs (each 0 or 1).
# @return           1 if any input is 1, else 0.

sub ORn {
    my @inputs = _variadic_inputs(@_);

    _throw( 'INVALID_ARITY', 'input', undef, "logic_gates: ORn requires at least 2 inputs\n" )
        if @inputs < 2;
    $_ = _bit( $_, "input" ) for @inputs;

    my $result = $inputs[0];
    for my $i ( 1 .. $#inputs ) {
        $result = OR( $result, $inputs[$i] );
    }
    return $result;
}

# XORn — N-input XOR gate (parity checker).
#
# Returns 1 if an ODD number of inputs are 1 (odd parity).
# Returns 0 if an EVEN number of inputs are 1 (even parity).
#
# XOR is often called the "parity gate": chaining XORs computes the parity
# of any bit string. Each stage answers "have I seen an odd total so far?".
#
# This is the building block for 8008/8080 parity flag computation:
#   P = NOT(XORn(@bits))  — the 8008 sets P=1 for even parity (even count of 1s)
#
# Truth table for 3-input XORn (same principle extends to N inputs):
#
#   A | B | C | XORn
#   --|---|---|------
#   0 | 0 | 0 |   0   (0 ones — even — XORn=0)
#   0 | 0 | 1 |   1   (1 one  — odd  — XORn=1)
#   0 | 1 | 0 |   1   (1 one  — odd  — XORn=1)
#   0 | 1 | 1 |   0   (2 ones — even — XORn=0)
#   1 | 0 | 0 |   1   (1 one  — odd  — XORn=1)
#   1 | 0 | 1 |   0   (2 ones — even — XORn=0)
#   1 | 1 | 0 |   0   (2 ones — even — XORn=0)
#   1 | 1 | 1 |   1   (3 ones — odd  — XORn=1)
#
# Hardware: a chain of 2-input XOR gates (N-1 gates for N inputs).
# Gate delay: N-1 (linear chain — real hardware uses a balanced tree for speed).
#
# @param  @inputs   One or more inputs (each 0 or 1).
# @return           1 if odd number of inputs are 1, else 0.

sub XORn {
    my @inputs = _variadic_inputs(@_);

    _throw( 'INVALID_ARITY', 'input', undef, "logic_gates: XORn requires at least 1 input\n" )
        if @inputs < 1;
    $_ = _bit( $_, "input" ) for @inputs;

    my $result = $inputs[0];
    for my $i ( 1 .. $#inputs ) {
        $result = XOR( $result, $inputs[$i] );
    }
    return $result;
}

# ============================================================================
# Sequential Logic — Circuits That Remember
# ============================================================================
#
# Everything above is "combinational": the output depends ONLY on current inputs.
# Sequential logic is different: the output depends on current inputs AND
# previous state.  Sequential circuits can REMEMBER.  This is the fundamental
# difference between a calculator and a computer.
#
# The key insight: by feeding a gate's output back to its own input, we create
# a circuit that can hold a value indefinitely.
#
# The memory hierarchy (simplest to most complex):
#
#   1. SR Latch      — remembers one bit (set/reset interface)
#   2. D Latch       — remembers one bit (data interface, transparent)
#   3. D Flip-Flop   — remembers one bit (data interface, edge-triggered)
#   4. Register      — remembers N bits (parallel flip-flops)
#   5. Shift Register — moves bits left/right on each clock
#   6. Counter       — counts up on each clock pulse

# ----------------------------------------------------------------------------
# State constructors
# ----------------------------------------------------------------------------

# new_flip_flop_state — create a fresh flip-flop state hash.
#
# A master-slave D flip-flop requires two latches, each tracking Q and Q-bar.
# We represent the state as a plain Perl hashref.
#
# @param  $master_q      Initial master Q (default 0).
# @param  $master_q_bar  Initial master Q-bar (default 1).
# @param  $slave_q       Initial slave Q (default 0).
# @param  $slave_q_bar   Initial slave Q-bar (default 1).
# @return                Hashref with keys: master_q, master_q_bar,
#                        slave_q, slave_q_bar.

sub new_flip_flop_state {
    my %args;
    my @a = _args( 'new_flip_flop_state', 0, 4, \@_ );
    if ( @a == 4 ) {
        %args = ( master_q => $a[0], master_q_bar => $a[1],
                  slave_q  => $a[2], slave_q_bar  => $a[3] );
        $args{$_} = _bit( $args{$_}, $_ ) for sort keys %args;
    } elsif ( @a != 0 ) {
        _throw( 'INVALID_ARITY', undef, undef,
            "logic_gates: new_flip_flop_state takes 0 or 4 argument(s), got " . scalar(@a) . "\n" );
    }
    return {
        master_q     => defined $args{master_q}     ? $args{master_q}     : 0,
        master_q_bar => defined $args{master_q_bar} ? $args{master_q_bar} : 1,
        slave_q      => defined $args{slave_q}      ? $args{slave_q}      : 0,
        slave_q_bar  => defined $args{slave_q_bar}  ? $args{slave_q_bar}  : 1,
    };
}

# new_counter_state — create a zeroed counter state.
#
# @param  $width   Number of bits in the counter (>= 1).
# @return          Hashref with keys: bits (arrayref of 0s), width.

# _validate_width -- a counter width is a positive decimal integer (no sign,
# no leading zero, no fraction) of at most MAX_WIDTH bits (CWE-400).
#
# The old check `$width < 1` numified its argument.  A reference numifies to
# its memory ADDRESS (~1e11), so new_counter_state([2]) passed the check and
# then tried to build (0) x 100_000_000_000 -- exhausting memory / hanging.
# "1e3" and 1e12 are likewise rejected by the string-shape test.  65536 bits is
# far beyond any real counter in this stack (the widest is 32).
use constant MAX_WIDTH => 65536;
sub _validate_width {
    my ($width) = @_;
    unless ( defined $width && !ref $width && $width =~ /\A[1-9][0-9]{0,4}\z/ && 0 + substr( $width, 0 ) <= MAX_WIDTH ) {
        my $d = _describe($width);
        _throw( 'INVALID_WIDTH', 'width', $d,
            "logic_gates: Counter width must be 1 to " . MAX_WIDTH . ", got $d\n" );
    }
    # Return a fresh integer built from the validated STRING form.  A dualvar
    # such as dualvar(-5, "3") is therefore the width 3 (the string decides,
    # as for bits); its numeric part is never used.
    return 0 + substr( $width, 0 );
}

sub new_counter_state {
    my ($width) = _args( 'new_counter_state', 1, 1, \@_ );
    $width = _validate_width($width);
    return {
        bits  => [ (0) x $width ],
        width => $width,
    };
}

# ============================================================================
# SR Latch — The Simplest Memory Element
# ============================================================================
#
# An SR (Set-Reset) latch is built from two NOR gates whose outputs feed back
# into each other's inputs, creating a stable feedback loop.
#
# Truth table:
#
#   S | R | Q (next) | Q-bar (next) | Action
#   --|---|----------|--------------|--------
#   0 | 0 |  hold    |  hold        | Remember (no change)
#   1 | 0 |  1       |  0           | Set (Q = 1)
#   0 | 1 |  0       |  1           | Reset (Q = 0)
#   1 | 1 |  0       |  0           | INVALID (Q-bar ≠ NOT Q)
#
# IMPLEMENTATION NOTE: We simulate the cross-coupled NOR feedback by
# iterating until the circuit reaches a stable state.  In real hardware
# this happens in nanoseconds.
#
# The NOR-latch equations:
#   Q     = NOR(Reset,  Q-bar)
#   Q-bar = NOR(Set,    Q)
#
# @param  $set    S input (0 or 1).
# @param  $reset  R input (0 or 1).
# @param  $q      Current Q output.
# @param  $q_bar  Current Q-bar output.
# @return         List ($new_q, $new_q_bar).

sub SRLatch {
    my ( $set, $reset, $q, $q_bar ) = _args( 'SRLatch', 4, 4, \@_ );

    $set = _bit($set, 'set');
    $reset = _bit($reset, 'reset');
    $q = _bit($q, 'q');
    $q_bar = _bit($q_bar, 'q_bar');

    my ( $cur_q, $cur_q_bar ) = ( $q, $q_bar );

    # Iterate up to 10 times to find stable state.
    # In practice, convergence is always achieved within 2 iterations.
    for ( 1 .. 10 ) {
        my $new_q     = NOR( $reset, $cur_q_bar );
        my $new_q_bar = NOR( $set,   $new_q     );
        last if $new_q == $cur_q && $new_q_bar == $cur_q_bar;
        $cur_q     = $new_q;
        $cur_q_bar = $new_q_bar;
    }

    return ( $cur_q, $cur_q_bar );
}

# ============================================================================
# D Latch — Taming the SR Latch
# ============================================================================
#
# The SR latch has the "invalid" state (S=1, R=1).  The D latch fixes this
# by deriving S and R from a single data input D and an enable signal:
#
#   Set   = D AND Enable
#   Reset = NOT(D) AND Enable
#
# Now S and R can never both be 1 simultaneously (because one is D and the
# other is NOT D).
#
# Behaviour:
#   Enable = 1 → Q follows D  (the latch is "transparent")
#   Enable = 0 → Q holds its last value (the latch is "opaque")
#
# @param  $data    D input (0 or 1).
# @param  $enable  Enable signal (0 or 1).
# @param  $q       Current Q.
# @param  $q_bar   Current Q-bar.
# @return          List ($new_q, $new_q_bar).

sub DLatch {
    my ( $data, $enable, $q, $q_bar ) = _args( 'DLatch', 4, 4, \@_ );

    $data = _bit($data, 'data');
    $enable = _bit($enable, 'enable');
    $q = _bit($q, 'q');
    $q_bar = _bit($q_bar, 'q_bar');

    my $set   = AND( $data,       $enable );
    my $reset = AND( NOT($data),  $enable );

    return SRLatch( $set, $reset, $q, $q_bar );
}

# ============================================================================
# D Flip-Flop — Edge-Triggered Memory
# ============================================================================
#
# The D flip-flop uses the MASTER-SLAVE technique: two D latches in series
# with opposite enable signals.
#
#   Clock = 1 (rising) → Master captures Data, Slave holds
#   Clock = 0 (falling) → Master holds, Slave receives from master
#
# The output changes only ONCE per clock cycle — at the clock's falling edge.
# This is called "edge triggering", and it is what makes synchronous digital
# design possible: every circuit in the whole CPU can update simultaneously
# on each clock edge.
#
# State representation: we use a hashref with keys:
#   master_q, master_q_bar, slave_q, slave_q_bar
#
# @param  $data   D input (0 or 1).
# @param  $clock  Clock signal (0 or 1).
# @param  $state  State hashref (or undef for initial state).
# @return         List ($q, $q_bar, $new_state_hashref).

# _validate_state -- a flip-flop state must be a plain hash holding four bits.
sub _validate_state {
    my ( $state, $who ) = @_;
    _throw( 'INVALID_SHAPE', 'state', undef, "logic_gates: $who state must be a hash reference\n" )
        unless ref $state eq 'HASH';
    for my $k (qw(master_q master_q_bar slave_q slave_q_bar)) {
        _bit( $state->{$k}, "state $k" );
    }
    return;
}

sub DFlipFlop {
    my ( $data, $clock, $state ) = _args( 'DFlipFlop', 2, 3, \@_ );

    $data = _bit($data, 'data');
    $clock = _bit($clock, 'clock');
    _validate_state( $state, 'DFlipFlop' ) if defined $state;

    $state //= new_flip_flop_state();

    # Master latch: enabled when clock = 1
    my ( $master_q, $master_q_bar ) = DLatch(
        $data, $clock,
        $state->{master_q}, $state->{master_q_bar}
    );

    # Slave latch: enabled when clock = 0 (NOT clock)
    my ( $slave_q, $slave_q_bar ) = DLatch(
        $master_q, NOT($clock),
        $state->{slave_q}, $state->{slave_q_bar}
    );

    my $new_state = {
        master_q     => $master_q,
        master_q_bar => $master_q_bar,
        slave_q      => $slave_q,
        slave_q_bar  => $slave_q_bar,
    };

    return ( $slave_q, $slave_q_bar, $new_state );
}

# ============================================================================
# Register — N Bits of Parallel Storage
# ============================================================================
#
# A register is N flip-flops sharing the same clock signal. Each flip-flop
# stores one bit independently.  A 64-bit CPU register is literally 64
# D flip-flops sharing a clock line.
#
# @param  $data   Arrayref of N bits (each 0 or 1).
# @param  $clock  The shared clock signal (0 or 1).
# @param  $state  Arrayref of N flip-flop state hashrefs, or undef.
# @return         List ($output_arrayref, $new_state_arrayref).

sub Register {
    my ( $data, $clock, $state ) = _args( 'Register', 2, 3, \@_ );

    $clock = _bit($clock, 'clock');
    _throw( 'INVALID_SHAPE', 'data', undef, "logic_gates: Register data must be an array reference\n" )
        unless ref $data eq 'ARRAY';
    _throw( 'INVALID_SHAPE', 'state', undef, "logic_gates: Register state must be an array reference\n" )
        if defined $state && ref $state ne 'ARRAY';

    my $n = scalar @{$data};
    _throw( 'INVALID_SHAPE', 'data', undef, "logic_gates: Register requires at least 1 bit of data\n" ) if $n == 0;

    unless ( defined $state ) {
        $state = [ map { new_flip_flop_state() } 1 .. $n ];
    }

    _throw( 'INVALID_SHAPE', 'state', undef, "logic_gates: Register data and state length mismatch\n" )
        if scalar @{$state} != $n;

    my ( @outputs, @new_states );
    for my $i ( 0 .. $n - 1 ) {
        my ( $q, undef, $ns ) = DFlipFlop( $data->[$i], $clock, $state->[$i] );
        push @outputs,    $q;
        push @new_states, $ns;
    }

    return ( \@outputs, \@new_states );
}

# ============================================================================
# Shift Register — Moving Bits Along a Chain
# ============================================================================
#
# A shift register is a chain of flip-flops where each one feeds the next.
# On each clock cycle, every bit shifts one position.
#
#   Left shift:   serialIn → [FF0] → [FF1] → [FF2] → serialOut
#   Right shift:  serialOut ← [FF0] ← [FF1] ← [FF2] ← serialIn
#
# Shift registers are used in:
#   - Serial-to-parallel conversion (USB, SPI)
#   - Pseudo-random number generation (LFSR)
#   - Delay lines
#
# @param  $serial_in   The bit entering the register (0 or 1).
# @param  $clock       Clock signal (0 or 1).
# @param  $state       Arrayref of N flip-flop state hashrefs.
# @param  $direction   'left' or 'right'.
# @return              List ($outputs_arrayref, $serial_out, $new_state_arrayref).

sub ShiftRegister {
    my ( $serial_in, $clock, $state, $direction ) = _args( 'ShiftRegister', 4, 4, \@_ );

    $serial_in = _bit($serial_in, 'serial_in');
    $clock = _bit($clock, 'clock');
    _throw( 'INVALID_SHAPE', 'direction', undef, "logic_gates: ShiftRegister direction must be 'left' or 'right'\n" )
        unless defined $direction && !ref $direction
            && ( $direction eq 'left' || $direction eq 'right' );
    _throw( 'INVALID_SHAPE', 'state', undef, "logic_gates: ShiftRegister requires non-empty state\n" )
        if !defined $state || ref $state ne 'ARRAY' || @{$state} == 0;

    my $n = scalar @{$state};

    # Capture current outputs (slave_q of each flip-flop) before shifting.
    my @current_outputs = map { $_->{slave_q} } @{$state};

    my ( @outputs, @new_states );

    if ( $direction eq 'left' ) {
        # Left shift: FF[0] gets serial_in, FF[i] gets old FF[i-1].
        # Serial out is the MSB (last element).
        for my $i ( 0 .. $n - 1 ) {
            my $data_in = ( $i == 0 ) ? $serial_in : $current_outputs[$i - 1];
            my ( $q, undef, $ns ) = DFlipFlop( $data_in, $clock, $state->[$i] );
            push @outputs,    $q;
            push @new_states, $ns;
        }
        return ( \@outputs, $current_outputs[$n - 1], \@new_states );
    }

    # Right shift: FF[n-1] gets serial_in, FF[i] gets old FF[i+1].
    # Serial out is the LSB (first element).
    for my $i ( reverse 0 .. $n - 1 ) {
        my $data_in = ( $i == $n - 1 ) ? $serial_in : $current_outputs[$i + 1];
        my ( $q, undef, $ns ) = DFlipFlop( $data_in, $clock, $state->[$i] );
        $outputs[$i]    = $q;
        $new_states[$i] = $ns;
    }
    return ( \@outputs, $current_outputs[0], \@new_states );
}

# ============================================================================
# Counter — A Self-Incrementing Register
# ============================================================================
#
# A binary counter adds 1 to its current value on each clock pulse.
# When all bits are 1, it wraps around to all 0s (overflow / rollover).
#
# The increment uses RIPPLE CARRY:
#
#   carry[0] = 1  (we are always adding 1)
#   For each bit i:
#     new_bit[i] = XOR(old_bit[i], carry[i])
#     carry[i+1] = AND(old_bit[i], carry[i])
#
# This is exactly how a 3rd-grade addition algorithm works, column by column.
#
# State representation: hashref with keys:
#   bits  — arrayref of bit values (LSB first, i.e., bits[0] = 2^0)
#   width — number of bits
#
# @param  $clock   Clock signal (0 or 1).
# @param  $reset   Asynchronous reset (1 = clear immediately).
# @param  $state   Counter state hashref.
# @return          List ($new_bits_arrayref, $new_state_hashref).

sub Counter {
    my ( $clock, $reset, $state ) = _args( 'Counter', 3, 3, \@_ );

    $clock = _bit($clock, 'clock');
    $reset = _bit($reset, 'reset');
    _throw( 'INVALID_SHAPE', 'state', undef, "logic_gates: Counter requires non-nil state\n" )
        unless defined $state && ref $state eq 'HASH';

    my $width = _validate_width( $state->{width} );
    _throw( 'INVALID_SHAPE', 'state', undef, "logic_gates: Counter state bits must be an array of width bits\n" )
        unless ref $state->{bits} eq 'ARRAY' && @{ $state->{bits} } == $width;

    # Asynchronous reset: immediately clear all bits.
    if ( $reset == 1 ) {
        my $new_bits = [ (0) x $width ];
        return ( $new_bits, { bits => $new_bits, width => $width } );
    }

    # On clock = 0, hold current value.
    if ( $clock == 0 ) {
        my @output = map { _bit( $_, 'state bit' ) } @{ $state->{bits} };
        return ( \@output, { bits => \@output, width => $width } );
    }

    # Increment using ripple carry (LSB first).
    my @new_bits;
    my $carry = 1;
    for my $i ( 0 .. $width - 1 ) {
        my $old_bit = $state->{bits}[$i];
        $new_bits[$i] = XOR( $old_bit, $carry );
        $carry        = AND( $old_bit, $carry );
    }

    return ( \@new_bits, { bits => \@new_bits, width => $width } );
}

1;

__END__

=head1 NAME

CodingAdventures::LogicGates - Combinational and sequential digital logic in pure Perl

=head1 SYNOPSIS

    use CodingAdventures::LogicGates qw(AND OR NOT XOR NAND NOR XNOR
                                         ANDn ORn
                                         SRLatch DLatch DFlipFlop
                                         Register ShiftRegister Counter
                                         new_flip_flop_state new_counter_state);

    # Combinational
    my $r = AND(1, 0);     # 0
    my $r = XOR(1, 0);     # 1
    my $r = NAND(1, 1);    # 0

    # Sequential — counter
    my $st = new_counter_state(4);
    my ($bits, $st2) = Counter(1, 0, $st);  # increment

=head1 DESCRIPTION

Pure-Perl implementation of all seven fundamental logic gates plus NAND-derived
variants and the classical sequential circuits: SR latch, D latch, D flip-flop,
register, shift register, and binary counter.  All state is represented as plain
hashrefs — no OOP needed.

=head1 VERSION

0.6.0

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
