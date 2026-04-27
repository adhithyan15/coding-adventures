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
# All functions also available as class methods.
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.02';

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
    AND OR NOT XOR NAND NOR XNOR
    NAND_NOT NAND_AND NAND_OR NAND_XOR
    ANDn ORn XORn
    SRLatch DLatch DFlipFlop
    Register ShiftRegister Counter
    new_flip_flop_state new_counter_state
);

# ============================================================================
# Internal: _validate_bit
# ============================================================================
#
# Ensure a value is a valid binary digit (0 or 1).
#
# In digital electronics, a "bit" is a signal that is either LOW (0) or HIGH (1).
# Anything else is undefined behaviour in hardware — a voltage outside the
# valid logic thresholds.  We enforce validity with a runtime die().
#
# @param  $value   The value to validate.
# @param  $name    The parameter name, for the error message.

sub _validate_bit {
    my ( $value, $name ) = @_;
    die "logic_gates: $name must be 0 or 1, got '$value'\n"
        if $value != 0 && $value != 1;
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
    my ( $a, $b ) = ( @_ == 3 ) ? ( $_[1], $_[2] ) : ( $_[0], $_[1] );
    _validate_bit( $a, 'a' );
    _validate_bit( $b, 'b' );
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
    my ( $a, $b ) = ( @_ == 3 ) ? ( $_[1], $_[2] ) : ( $_[0], $_[1] );
    _validate_bit( $a, 'a' );
    _validate_bit( $b, 'b' );
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
    my $a = ( @_ == 2 ) ? $_[1] : $_[0];
    _validate_bit( $a, 'a' );
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
    my ( $a, $b ) = ( @_ == 3 ) ? ( $_[1], $_[2] ) : ( $_[0], $_[1] );
    _validate_bit( $a, 'a' );
    _validate_bit( $b, 'b' );
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
    my ( $a, $b ) = ( @_ == 3 ) ? ( $_[1], $_[2] ) : ( $_[0], $_[1] );
    _validate_bit( $a, 'a' );
    _validate_bit( $b, 'b' );
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
    my ( $a, $b ) = ( @_ == 3 ) ? ( $_[1], $_[2] ) : ( $_[0], $_[1] );
    _validate_bit( $a, 'a' );
    _validate_bit( $b, 'b' );
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
    my ( $a, $b ) = ( @_ == 3 ) ? ( $_[1], $_[2] ) : ( $_[0], $_[1] );
    _validate_bit( $a, 'a' );
    _validate_bit( $b, 'b' );
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
    my $a = ( @_ == 2 ) ? $_[1] : $_[0];
    _validate_bit( $a, 'a' );
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
    my ( $a, $b ) = ( @_ == 3 ) ? ( $_[1], $_[2] ) : ( $_[0], $_[1] );
    _validate_bit( $a, 'a' );
    _validate_bit( $b, 'b' );
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
    my ( $a, $b ) = ( @_ == 3 ) ? ( $_[1], $_[2] ) : ( $_[0], $_[1] );
    _validate_bit( $a, 'a' );
    _validate_bit( $b, 'b' );
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
    my ( $a, $b ) = ( @_ == 3 ) ? ( $_[1], $_[2] ) : ( $_[0], $_[1] );
    _validate_bit( $a, 'a' );
    _validate_bit( $b, 'b' );
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

sub ANDn {
    my @inputs = ( ref $_[0] eq 'ARRAY' ) ? @{ $_[0] }  # array-ref style
               : ( @_ > 0 && !ref $_[0] && $_[0] =~ /::/ ) ? @_[1..$#_]  # method call
               : @_;

    die "logic_gates: ANDn requires at least 2 inputs\n" if @inputs < 2;
    _validate_bit( $_, "input" ) for @inputs;

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
    my @inputs = ( ref $_[0] eq 'ARRAY' ) ? @{ $_[0] }
               : ( @_ > 0 && !ref $_[0] && $_[0] =~ /::/ ) ? @_[1..$#_]
               : @_;

    die "logic_gates: ORn requires at least 2 inputs\n" if @inputs < 2;
    _validate_bit( $_, "input" ) for @inputs;

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
    my @inputs = ( ref $_[0] eq 'ARRAY' ) ? @{ $_[0] }
               : ( @_ > 0 && !ref $_[0] && $_[0] =~ /::/ ) ? @_[1..$#_]
               : @_;

    die "logic_gates: XORn requires at least 1 input\n" if @inputs < 1;
    _validate_bit( $_, "input" ) for @inputs;

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
    if ( @_ == 5 ) {
        # method call: (class, mq, mqb, sq, sqb)
        %args = ( master_q => $_[1], master_q_bar => $_[2],
                  slave_q  => $_[3], slave_q_bar  => $_[4] );
    } elsif ( @_ == 4 ) {
        %args = ( master_q => $_[0], master_q_bar => $_[1],
                  slave_q  => $_[2], slave_q_bar  => $_[3] );
    } else {
        %args = ();
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

sub new_counter_state {
    my $width = ( @_ == 2 ) ? $_[1] : $_[0];
    die "logic_gates: Counter width must be at least 1\n" if $width < 1;
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
    my ( $set, $reset, $q, $q_bar ) = @_;
    # Peel off class name if called as method
    if ( $set =~ /::/ ) {
        ( $set, $reset, $q, $q_bar ) = ( $reset, $q, $q_bar, $_[4] );
    }

    _validate_bit( $set,   'set'   );
    _validate_bit( $reset, 'reset' );
    _validate_bit( $q,     'q'     );
    _validate_bit( $q_bar, 'q_bar' );

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
    my ( $data, $enable, $q, $q_bar ) = @_;
    if ( $data =~ /::/ ) {
        ( $data, $enable, $q, $q_bar ) = ( $enable, $q, $q_bar, $_[4] );
    }

    _validate_bit( $data,   'data'   );
    _validate_bit( $enable, 'enable' );
    _validate_bit( $q,      'q'      );
    _validate_bit( $q_bar,  'q_bar'  );

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

sub DFlipFlop {
    my ( $data, $clock, $state ) = @_;
    if ( $data =~ /::/ ) {
        ( $data, $clock, $state ) = ( $clock, $state, $_[3] );
    }

    die "logic_gates: data must be 0 or 1, got '$data'\n"
        if $data != 0 && $data != 1;
    die "logic_gates: clock must be 0 or 1, got '$clock'\n"
        if $clock != 0 && $clock != 1;

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
    my ( $data, $clock, $state ) = @_;
    if ( ref($data) eq '' && $data =~ /::/ ) {
        ( $data, $clock, $state ) = ( $clock, $state, $_[3] );
    }

    die "logic_gates: clock must be 0 or 1, got '$clock'\n"
        if $clock != 0 && $clock != 1;

    my $n = scalar @{$data};
    die "logic_gates: Register requires at least 1 bit of data\n" if $n == 0;

    unless ( defined $state ) {
        $state = [ map { new_flip_flop_state() } 1 .. $n ];
    }

    die "logic_gates: Register data and state length mismatch\n"
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
    my ( $serial_in, $clock, $state, $direction ) = @_;
    if ( ref($serial_in) eq '' && $serial_in =~ /::/ ) {
        ( $serial_in, $clock, $state, $direction ) = ( $clock, $state, $direction, $_[4] );
    }

    die "logic_gates: serial_in must be 0 or 1, got '$serial_in'\n"
        if $serial_in != 0 && $serial_in != 1;
    die "logic_gates: clock must be 0 or 1, got '$clock'\n"
        if $clock != 0 && $clock != 1;
    die "logic_gates: ShiftRegister direction must be 'left' or 'right'\n"
        if $direction ne 'left' && $direction ne 'right';
    die "logic_gates: ShiftRegister requires non-empty state\n"
        if !defined $state || @{$state} == 0;

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
    my ( $clock, $reset, $state ) = @_;
    if ( $clock =~ /::/ ) {
        ( $clock, $reset, $state ) = ( $reset, $state, $_[3] );
    }

    die "logic_gates: clock must be 0 or 1, got '$clock'\n"
        if $clock != 0 && $clock != 1;
    die "logic_gates: reset must be 0 or 1, got '$reset'\n"
        if $reset != 0 && $reset != 1;
    die "logic_gates: Counter requires non-nil state\n"
        unless defined $state;

    my $width = $state->{width};
    die "logic_gates: Counter width must be at least 1\n" if $width < 1;

    # Asynchronous reset: immediately clear all bits.
    if ( $reset == 1 ) {
        my $new_bits = [ (0) x $width ];
        return ( $new_bits, { bits => $new_bits, width => $width } );
    }

    # On clock = 0, hold current value.
    if ( $clock == 0 ) {
        my @output = @{ $state->{bits} };
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

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
