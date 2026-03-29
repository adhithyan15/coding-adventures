use strict;
use warnings;
use Test2::V0;

# ---------------------------------------------------------------------------
# Load the module under test.
# ---------------------------------------------------------------------------
ok( eval { require CodingAdventures::Transistors; 1 }, 'CodingAdventures::Transistors loads' );

use CodingAdventures::Transistors qw(
    nmos  pmos  npn  pnp
    cmos_not  cmos_nand  cmos_nor  cmos_and  cmos_or  cmos_xor  cmos_xnor
    ttl_not   ttl_nand
    MOSFET_CUTOFF  MOSFET_LINEAR  MOSFET_SATURATION
    BJT_CUTOFF     BJT_ACTIVE     BJT_SATURATION
);

# ===========================================================================
# 1. Operating region constants are defined
# ===========================================================================

is( MOSFET_CUTOFF,     'cutoff',     'MOSFET_CUTOFF constant' );
is( MOSFET_LINEAR,     'linear',     'MOSFET_LINEAR constant' );
is( MOSFET_SATURATION, 'saturation', 'MOSFET_SATURATION constant' );
is( BJT_CUTOFF,        'cutoff',     'BJT_CUTOFF constant' );
is( BJT_ACTIVE,        'active',     'BJT_ACTIVE constant' );
is( BJT_SATURATION,    'saturation', 'BJT_SATURATION constant' );

# ===========================================================================
# 2. NMOS — N-channel MOSFET conductance
#
# NMOS turns ON when Vgs >= Vth.  Threshold = 0.4 V (default 180 nm).
# ===========================================================================

# Vgs > Vth → conducting
is( nmos( 1.8, 0.4 ), 1, 'nmos: Vgs=1.8 > Vth=0.4 → ON' );
is( nmos( 0.5, 0.4 ), 1, 'nmos: Vgs=0.5 > Vth=0.4 → ON' );
is( nmos( 0.4, 0.4 ), 1, 'nmos: Vgs=0.4 = Vth=0.4 → ON (boundary)' );

# Vgs < Vth → not conducting
is( nmos( 0.0, 0.4 ), 0, 'nmos: Vgs=0.0 < Vth=0.4 → OFF' );
is( nmos( 0.3, 0.4 ), 0, 'nmos: Vgs=0.3 < Vth=0.4 → OFF' );
is( nmos( -1.0, 0.4 ), 0, 'nmos: Vgs=-1.0 < Vth → OFF' );

# Default Vth (0.4 V) is used when second argument omitted
is( nmos( 1.0 ), 1, 'nmos: Vgs=1.0 with default Vth → ON' );
is( nmos( 0.1 ), 0, 'nmos: Vgs=0.1 with default Vth → OFF' );

# ===========================================================================
# 3. PMOS — P-channel MOSFET conductance
#
# PMOS turns ON when |Vgs| >= Vth.  Typically Vgs is negative (gate below source).
# ===========================================================================

# |Vgs| >= Vth → conducting
is( pmos( -3.3, 0.4 ), 1, 'pmos: Vgs=-3.3, |Vgs|=3.3 >= Vth=0.4 → ON' );
is( pmos( -0.4, 0.4 ), 1, 'pmos: Vgs=-0.4, |Vgs|=0.4 >= Vth → ON (boundary)' );
is( pmos( 0.0,  0.4 ), 0, 'pmos: Vgs=0.0, |Vgs|=0 < Vth → OFF' );
is( pmos( 0.3,  0.4 ), 0, 'pmos: |Vgs|=0.3 < Vth=0.4 → OFF' );
is( pmos( -0.1, 0.4 ), 0, 'pmos: |Vgs|=0.1 < Vth=0.4 → OFF' );

# NMOS/PMOS complementary behaviour:
# Input HIGH (Vgs_n = 3.3 V, Vgs_p = 3.3 - 3.3 = 0 V):
#   NMOS: Vgs=3.3 → ON;  PMOS: |Vgs|=0 → OFF
is( nmos( 3.3, 0.4 ), 1, 'CMOS complementarity: NMOS ON for HIGH input' );
is( pmos( 0.0, 0.4 ), 0, 'CMOS complementarity: PMOS OFF for HIGH input' );
# Input LOW (Vgs_n = 0 V, Vgs_p = 0 - 3.3 = -3.3 V):
#   NMOS: Vgs=0 → OFF; PMOS: |Vgs|=3.3 → ON
is( nmos( 0.0, 0.4 ), 0, 'CMOS complementarity: NMOS OFF for LOW input' );
is( pmos(-3.3, 0.4 ), 1, 'CMOS complementarity: PMOS ON for LOW input' );

# ===========================================================================
# 4. NPN BJT conductance
# ===========================================================================

is( npn( 0.001, 100 ), 1, 'npn: Ib=1mA → ON' );
is( npn( 1e-6,  100 ), 1, 'npn: Ib=1uA → ON (any positive Ib)' );
is( npn( 0.0,   100 ), 0, 'npn: Ib=0 → OFF' );
is( npn( -0.001, 100), 0, 'npn: Ib<0 → OFF' );

# Default beta
is( npn( 0.001 ), 1, 'npn with default beta: Ib=1mA → ON' );
is( npn( 0.0 ),   0, 'npn with default beta: Ib=0 → OFF' );

# ===========================================================================
# 5. PNP BJT conductance
# ===========================================================================

is( pnp( 0.001, 100 ), 1, 'pnp: |Ib|=1mA → ON' );
is( pnp( 0.0,   100 ), 0, 'pnp: |Ib|=0 → OFF' );
is( pnp( 0.001 ),      1, 'pnp with default beta → ON' );
is( pnp( 0.0 ),        0, 'pnp with default beta → OFF' );

# ===========================================================================
# 6. CMOS NOT (Inverter) — full truth table
# ===========================================================================

is( cmos_not(0), 1, 'cmos_not(0) = 1' );
is( cmos_not(1), 0, 'cmos_not(1) = 0' );

# Double inversion returns original value.
is( cmos_not( cmos_not(0) ), 0, 'cmos_not(cmos_not(0)) = 0' );
is( cmos_not( cmos_not(1) ), 1, 'cmos_not(cmos_not(1)) = 1' );

# ===========================================================================
# 7. CMOS NAND — full truth table
# ===========================================================================

is( cmos_nand(0, 0), 1, 'cmos_nand(0,0) = 1' );
is( cmos_nand(0, 1), 1, 'cmos_nand(0,1) = 1' );
is( cmos_nand(1, 0), 1, 'cmos_nand(1,0) = 1' );
is( cmos_nand(1, 1), 0, 'cmos_nand(1,1) = 0' );

# NAND = NOT(AND): verify against expected AND truth table.
is( cmos_nand(1, 1), 0, 'NAND(1,1) = NOT(AND(1,1)) = NOT(1) = 0' );

# ===========================================================================
# 8. CMOS NOR — full truth table
# ===========================================================================

is( cmos_nor(0, 0), 1, 'cmos_nor(0,0) = 1' );
is( cmos_nor(0, 1), 0, 'cmos_nor(0,1) = 0' );
is( cmos_nor(1, 0), 0, 'cmos_nor(1,0) = 0' );
is( cmos_nor(1, 1), 0, 'cmos_nor(1,1) = 0' );

# ===========================================================================
# 9. CMOS AND — full truth table (NAND + NOT)
# ===========================================================================

is( cmos_and(0, 0), 0, 'cmos_and(0,0) = 0' );
is( cmos_and(0, 1), 0, 'cmos_and(0,1) = 0' );
is( cmos_and(1, 0), 0, 'cmos_and(1,0) = 0' );
is( cmos_and(1, 1), 1, 'cmos_and(1,1) = 1' );

# AND = NOT(NAND): sanity check
for my $a (0, 1) {
    for my $b (0, 1) {
        is( cmos_and($a, $b), cmos_not( cmos_nand($a, $b) ),
            "cmos_and($a,$b) = NOT(NAND($a,$b))" );
    }
}

# ===========================================================================
# 10. CMOS OR — full truth table (NOR + NOT)
# ===========================================================================

is( cmos_or(0, 0), 0, 'cmos_or(0,0) = 0' );
is( cmos_or(0, 1), 1, 'cmos_or(0,1) = 1' );
is( cmos_or(1, 0), 1, 'cmos_or(1,0) = 1' );
is( cmos_or(1, 1), 1, 'cmos_or(1,1) = 1' );

# OR = NOT(NOR): sanity check
for my $a (0, 1) {
    for my $b (0, 1) {
        is( cmos_or($a, $b), cmos_not( cmos_nor($a, $b) ),
            "cmos_or($a,$b) = NOT(NOR($a,$b))" );
    }
}

# ===========================================================================
# 11. CMOS XOR — full truth table (4-NAND construction)
# ===========================================================================

is( cmos_xor(0, 0), 0, 'cmos_xor(0,0) = 0' );
is( cmos_xor(0, 1), 1, 'cmos_xor(0,1) = 1' );
is( cmos_xor(1, 0), 1, 'cmos_xor(1,0) = 1' );
is( cmos_xor(1, 1), 0, 'cmos_xor(1,1) = 0' );

# XOR is commutative: cmos_xor(A,B) = cmos_xor(B,A)
for my $a (0, 1) {
    for my $b (0, 1) {
        is( cmos_xor($a, $b), cmos_xor($b, $a),
            "cmos_xor($a,$b) is commutative" );
    }
}

# XOR(A,A) = 0 (same inputs cancel)
is( cmos_xor(0, 0), 0, 'cmos_xor(0,0): same inputs → 0' );
is( cmos_xor(1, 1), 0, 'cmos_xor(1,1): same inputs → 0' );

# ===========================================================================
# 12. CMOS XNOR — full truth table (XOR + inverter)
# ===========================================================================

is( cmos_xnor(0, 0), 1, 'cmos_xnor(0,0) = 1' );
is( cmos_xnor(0, 1), 0, 'cmos_xnor(0,1) = 0' );
is( cmos_xnor(1, 0), 0, 'cmos_xnor(1,0) = 0' );
is( cmos_xnor(1, 1), 1, 'cmos_xnor(1,1) = 1' );

# XNOR = NOT(XOR): verify against truth table
for my $a (0, 1) {
    for my $b (0, 1) {
        is( cmos_xnor($a, $b), cmos_not( cmos_xor($a, $b) ),
            "cmos_xnor($a,$b) = NOT(XOR($a,$b))" );
    }
}

# XNOR(A,A) = 1 (same inputs are always equal)
is( cmos_xnor(0, 0), 1, 'cmos_xnor(0,0): same inputs → 1' );
is( cmos_xnor(1, 1), 1, 'cmos_xnor(1,1): same inputs → 1' );

# ===========================================================================
# 13. TTL NAND — full truth table
# ===========================================================================


is( ttl_nand(0, 0), 1, 'ttl_nand(0,0) = 1' );
is( ttl_nand(0, 1), 1, 'ttl_nand(0,1) = 1' );
is( ttl_nand(1, 0), 1, 'ttl_nand(1,0) = 1' );
is( ttl_nand(1, 1), 0, 'ttl_nand(1,1) = 0' );

# TTL NAND agrees with CMOS NAND for all input combinations.
for my $a (0, 1) {
    for my $b (0, 1) {
        is( ttl_nand($a, $b), cmos_nand($a, $b),
            "ttl_nand($a,$b) agrees with cmos_nand($a,$b)" );
    }
}

# ===========================================================================
# 14. TTL NOT (RTL inverter) — full truth table
# ===========================================================================

is( ttl_not(0), 1, 'ttl_not(0) = 1' );
is( ttl_not(1), 0, 'ttl_not(1) = 0' );

# Agrees with CMOS NOT.
is( ttl_not(0), cmos_not(0), 'ttl_not(0) agrees with cmos_not(0)' );
is( ttl_not(1), cmos_not(1), 'ttl_not(1) agrees with cmos_not(1)' );

# ===========================================================================
# 15. Invalid input handling
# ===========================================================================

ok( eval { cmos_not(2);       0 } // 1, 'cmos_not(2) dies' );
ok( eval { cmos_nand(0, 2);   0 } // 1, 'cmos_nand(0,2) dies' );
ok( eval { cmos_nor(-1, 0);   0 } // 1, 'cmos_nor(-1,0) dies' );
ok( eval { cmos_and(0, 3);    0 } // 1, 'cmos_and(0,3) dies' );
ok( eval { cmos_or(2, 0);     0 } // 1, 'cmos_or(2,0) dies' );
ok( eval { cmos_xor(2, 0);    0 } // 1, 'cmos_xor(2,0) dies' );
ok( eval { cmos_xor(0, -1);   0 } // 1, 'cmos_xor(0,-1) dies' );
ok( eval { cmos_xnor(2, 0);   0 } // 1, 'cmos_xnor(2,0) dies' );
ok( eval { cmos_xnor(0, -1);  0 } // 1, 'cmos_xnor(0,-1) dies' );
ok( eval { ttl_nand(0, 2);    0 } // 1, 'ttl_nand(0,2) dies' );
ok( eval { ttl_not(2);        0 } // 1, 'ttl_not(2) dies' );

done_testing;
