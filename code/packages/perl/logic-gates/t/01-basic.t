use strict;
use warnings;
use Test2::V0;

# ---------------------------------------------------------------------------
# Load the module under test.
# ---------------------------------------------------------------------------
ok( eval { require CodingAdventures::LogicGates; 1 }, 'CodingAdventures::LogicGates loads' );

use CodingAdventures::LogicGates qw(
    AND OR NOT XOR NAND NOR XNOR
    NAND_NOT NAND_AND NAND_OR NAND_XOR
    ANDn ORn
    SRLatch DLatch DFlipFlop
    Register ShiftRegister Counter
    new_flip_flop_state new_counter_state
);

# ===========================================================================
# 1. AND gate — full truth table
# ===========================================================================

is( AND(0, 0), 0, 'AND(0,0) = 0' );
is( AND(0, 1), 0, 'AND(0,1) = 0' );
is( AND(1, 0), 0, 'AND(1,0) = 0' );
is( AND(1, 1), 1, 'AND(1,1) = 1' );

# ===========================================================================
# 2. OR gate — full truth table
# ===========================================================================

is( OR(0, 0), 0, 'OR(0,0) = 0' );
is( OR(0, 1), 1, 'OR(0,1) = 1' );
is( OR(1, 0), 1, 'OR(1,0) = 1' );
is( OR(1, 1), 1, 'OR(1,1) = 1' );

# ===========================================================================
# 3. NOT gate
# ===========================================================================

is( NOT(0), 1, 'NOT(0) = 1' );
is( NOT(1), 0, 'NOT(1) = 0' );

# ===========================================================================
# 4. XOR gate — full truth table
# ===========================================================================

is( XOR(0, 0), 0, 'XOR(0,0) = 0' );
is( XOR(0, 1), 1, 'XOR(0,1) = 1' );
is( XOR(1, 0), 1, 'XOR(1,0) = 1' );
is( XOR(1, 1), 0, 'XOR(1,1) = 0' );

# ===========================================================================
# 5. NAND gate — full truth table
# ===========================================================================

is( NAND(0, 0), 1, 'NAND(0,0) = 1' );
is( NAND(0, 1), 1, 'NAND(0,1) = 1' );
is( NAND(1, 0), 1, 'NAND(1,0) = 1' );
is( NAND(1, 1), 0, 'NAND(1,1) = 0' );

# ===========================================================================
# 6. NOR gate — full truth table
# ===========================================================================

is( NOR(0, 0), 1, 'NOR(0,0) = 1' );
is( NOR(0, 1), 0, 'NOR(0,1) = 0' );
is( NOR(1, 0), 0, 'NOR(1,0) = 0' );
is( NOR(1, 1), 0, 'NOR(1,1) = 0' );

# ===========================================================================
# 7. XNOR gate — full truth table
# ===========================================================================

is( XNOR(0, 0), 1, 'XNOR(0,0) = 1' );
is( XNOR(0, 1), 0, 'XNOR(0,1) = 0' );
is( XNOR(1, 0), 0, 'XNOR(1,0) = 0' );
is( XNOR(1, 1), 1, 'XNOR(1,1) = 1' );

# ===========================================================================
# 8. NAND-derived gates (universality proofs)
# ===========================================================================

# NAND_NOT agrees with NOT on all inputs.
is( NAND_NOT(0), NOT(0), 'NAND_NOT(0) matches NOT(0)' );
is( NAND_NOT(1), NOT(1), 'NAND_NOT(1) matches NOT(1)' );

# NAND_AND agrees with AND on all input combinations.
for my $a (0, 1) {
    for my $b (0, 1) {
        is( NAND_AND($a, $b), AND($a, $b), "NAND_AND($a,$b) matches AND($a,$b)" );
    }
}

# NAND_OR agrees with OR.
for my $a (0, 1) {
    for my $b (0, 1) {
        is( NAND_OR($a, $b), OR($a, $b), "NAND_OR($a,$b) matches OR($a,$b)" );
    }
}

# NAND_XOR agrees with XOR.
for my $a (0, 1) {
    for my $b (0, 1) {
        is( NAND_XOR($a, $b), XOR($a, $b), "NAND_XOR($a,$b) matches XOR($a,$b)" );
    }
}

# ===========================================================================
# 9. Multi-input gates: ANDn and ORn
# ===========================================================================

is( ANDn(1, 1, 1), 1, 'ANDn(1,1,1) = 1' );
is( ANDn(1, 1, 0), 0, 'ANDn(1,1,0) = 0' );
is( ANDn(0, 1, 1), 0, 'ANDn(0,1,1) = 0' );
is( ANDn(1, 1, 1, 1), 1, 'ANDn(1,1,1,1) = 1' );
is( ANDn(1, 0, 1, 1), 0, 'ANDn(1,0,1,1) = 0' );

is( ORn(0, 0, 0), 0, 'ORn(0,0,0) = 0' );
is( ORn(0, 0, 1), 1, 'ORn(0,0,1) = 1' );
is( ORn(1, 0, 0), 1, 'ORn(1,0,0) = 1' );
is( ORn(1, 1, 1), 1, 'ORn(1,1,1) = 1' );
is( ORn(0, 0, 0, 0), 0, 'ORn(0,0,0,0) = 0' );

# ===========================================================================
# 10. Input validation — invalid inputs should die
# ===========================================================================

ok( eval { AND(2, 1); 0 } // 1,  'AND(2,1) dies on invalid input' );
ok( eval { OR(0, 3); 0 }  // 1,  'OR(0,3) dies on invalid input' );
ok( eval { NOT(2); 0 }    // 1,  'NOT(2) dies on invalid input' );
ok( eval { XOR(-1, 0); 0} // 1,  'XOR(-1,0) dies on invalid input' );

# ===========================================================================
# 11. SR Latch — truth table
# ===========================================================================

# Set (S=1, R=0) → Q=1, Q-bar=0
my ( $q, $qb ) = SRLatch( 1, 0, 0, 1 );
is( $q,  1, 'SRLatch SET: Q=1' );
is( $qb, 0, 'SRLatch SET: Q-bar=0' );

# Reset (S=0, R=1) → Q=0, Q-bar=1
( $q, $qb ) = SRLatch( 0, 1, 1, 0 );
is( $q,  0, 'SRLatch RESET: Q=0' );
is( $qb, 1, 'SRLatch RESET: Q-bar=1' );

# Hold (S=0, R=0) with Q=1 → Q=1 (remembers)
( $q, $qb ) = SRLatch( 0, 0, 1, 0 );
is( $q,  1, 'SRLatch HOLD Q=1: Q stays 1' );

# Hold (S=0, R=0) with Q=0 → Q=0 (remembers)
( $q, $qb ) = SRLatch( 0, 0, 0, 1 );
is( $q,  0, 'SRLatch HOLD Q=0: Q stays 0' );

# ===========================================================================
# 12. D Latch
# ===========================================================================

# Enable=1, Data=1 → Q=1 (transparent: latch writes D)
( $q, $qb ) = DLatch( 1, 1, 0, 1 );
is( $q, 1, 'DLatch enable=1 data=1: Q=1' );

# Enable=1, Data=0 → Q=0
( $q, $qb ) = DLatch( 0, 1, 1, 0 );
is( $q, 0, 'DLatch enable=1 data=0: Q=0' );

# Enable=0 → Q holds its old value regardless of Data
( $q, $qb ) = DLatch( 1, 0, 0, 1 );
is( $q, 0, 'DLatch enable=0 data=1: Q holds 0' );

( $q, $qb ) = DLatch( 0, 0, 1, 0 );
is( $q, 1, 'DLatch enable=0 data=0: Q holds 1' );

# ===========================================================================
# 13. D Flip-Flop — edge-triggered behaviour
# ===========================================================================

# Initial state: all zero.
my $state = new_flip_flop_state();

# Clock = 1 (rising): master captures Data, slave holds.
my ( $out, $outb, $st2 ) = DFlipFlop( 1, 1, $state );

# Clock = 0 (falling): slave updates to master's captured value.
my ( $out2, $outb2, $st3 ) = DFlipFlop( 1, 0, $st2 );
is( $out2, 1, 'DFlipFlop: D=1, after full cycle, Q=1' );

# New data on D=0, clock rising then falling.
my $st4;
( undef, undef, $st4 ) = DFlipFlop( 0, 1, $st3 );
my $final_q;
( $final_q, undef, undef ) = DFlipFlop( 0, 0, $st4 );
is( $final_q, 0, 'DFlipFlop: D=0, after full cycle, Q=0' );

# ===========================================================================
# 14. Register — N-bit parallel storage
# ===========================================================================

# Load [1,0,1,0] into a 4-bit register.
my $data = [1, 0, 1, 0];
my ( $out_arr, $reg_st ) = Register( $data, 1, undef );  # clock high (capture)
( $out_arr, $reg_st ) = Register( $data, 0, $reg_st );   # clock low (latch)
is( $out_arr, [1, 0, 1, 0], 'Register stores [1,0,1,0]' );

# Load [0,1,1,0].
$data = [0, 1, 1, 0];
( $out_arr, $reg_st ) = Register( $data, 1, $reg_st );
( $out_arr, $reg_st ) = Register( $data, 0, $reg_st );
is( $out_arr, [0, 1, 1, 0], 'Register stores [0,1,1,0]' );

# ===========================================================================
# 15. Counter — binary increment
# ===========================================================================

my $cnt_st = new_counter_state(4);  # 4-bit counter, starts at 0000

# Clock one tick.
my ( $bits, $cnt_st2 ) = Counter( 1, 0, $cnt_st );
is( $bits, [1, 0, 0, 0], 'Counter tick 1: bits = [1,0,0,0] (LSB first = 1)' );

# Clock another tick.
my ( $bits2, $cnt_st3 ) = Counter( 1, 0, $cnt_st2 );
is( $bits2, [0, 1, 0, 0], 'Counter tick 2: bits = [0,1,0,0] (= 2)' );

# Reset clears to all zeros.
my ( $bits_reset, undef ) = Counter( 1, 1, $cnt_st3 );
is( $bits_reset, [0, 0, 0, 0], 'Counter reset: all zeros' );

# Clock=0 → hold
my ( $bits_hold, undef ) = Counter( 0, 0, $cnt_st2 );
is( $bits_hold, [1, 0, 0, 0], 'Counter clock=0 holds value' );

# ===========================================================================
# 16. Shift Register — left shift
# ===========================================================================

# Initialise a 4-bit shift register with all zeros.
my @sr_state = map { new_flip_flop_state() } 1 .. 4;

# Shift in a 1 from the left.
my ( $sr_out, $serial_out, $sr_st2 );
( $sr_out, $serial_out, $sr_st2 ) = ShiftRegister( 1, 1, \@sr_state, 'left' );
( $sr_out, $serial_out, $sr_st2 ) = ShiftRegister( 1, 0, $sr_st2,    'left' );
is( $sr_out->[0], 1, 'ShiftRegister left: first bit after one shift = 1' );

# Shift in another 1.
my $sr_st3;
( $sr_out, $serial_out, $sr_st3 ) = ShiftRegister( 1, 1, $sr_st2, 'left' );
( $sr_out, $serial_out, $sr_st3 ) = ShiftRegister( 1, 0, $sr_st3, 'left' );
is( $sr_out->[0], 1, 'ShiftRegister left: bit 0 still 1 after second shift' );
is( $sr_out->[1], 1, 'ShiftRegister left: bit 1 = 1 after second shift' );

done_testing;
