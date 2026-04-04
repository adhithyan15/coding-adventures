use strict;
use warnings;
use Test2::V0;

use CodingAdventures::GF256 qw(add subtract multiply divide power inverse);

# ============================================================================
# Constants check
# ============================================================================
# Verify the exported constants match the spec.

subtest 'constants' => sub {
    is( $CodingAdventures::GF256::ZERO, 0,     'ZERO = 0' );
    is( $CodingAdventures::GF256::ONE,  1,     'ONE = 1' );
    is( $CodingAdventures::GF256::PRIMITIVE_POLYNOMIAL, 0x11D,
        'PRIMITIVE_POLYNOMIAL = 0x11D' );
};

# ============================================================================
# ALOG / LOG table sanity checks
# ============================================================================
# These tables are package-level state; we verify their key properties.
#
# Key invariants:
#   1. ALOG[0] = 1  (g^0 = 1)
#   2. ALOG[1] = 2  (generator is x = 2)
#   3. ALOG[255] = 1  (g^255 = g^0 = 1 in a group of order 255)
#   4. LOG[1] = 0    (log base g of 1 is 0)
#   5. LOG[2] = 1    (log base g of g is 1)
#   6. Every non-zero element appears exactly once in ALOG[0..254]

subtest 'LOG and ALOG table invariants' => sub {
    is( $CodingAdventures::GF256::ALOG[0],   1, 'ALOG[0] = 1' );
    is( $CodingAdventures::GF256::ALOG[1],   2, 'ALOG[1] = 2 (generator)' );
    is( $CodingAdventures::GF256::ALOG[255], 1, 'ALOG[255] = 1' );

    is( $CodingAdventures::GF256::LOG[1], 0, 'LOG[1] = 0' );
    is( $CodingAdventures::GF256::LOG[2], 1, 'LOG[2] = 1' );

    # Every element 1..255 should appear exactly once in ALOG[0..254]
    my %seen;
    for my $i (0 .. 254) {
        my $v = $CodingAdventures::GF256::ALOG[$i];
        ok( $v >= 1 && $v <= 255, "ALOG[$i] in range 1..255" );
        $seen{$v}++;
    }
    is( scalar(keys %seen), 255, 'all 255 non-zero elements appear in ALOG' );
    for my $k (keys %seen) {
        is( $seen{$k}, 1, "element $k appears exactly once" );
    }
};

# ============================================================================
# add / subtract
# ============================================================================
# In GF(2^8) these are identical (both are XOR).

subtest 'add' => sub {
    # XOR properties
    is( add(0,   0),   0,    'add: 0+0=0' );
    is( add(1,   0),   1,    'add: 1+0=1' );
    is( add(0,   1),   1,    'add: 0+1=1' );
    is( add(1,   1),   0,    'add: 1+1=0 (characteristic 2)' );

    # Specific byte values
    is( add(0x53, 0xCA), 0x53 ^ 0xCA, 'add: 0x53 + 0xCA = XOR' );

    # Adding an element to itself gives zero
    is( add(0xFF, 0xFF), 0,   'add: a+a=0 for all a' );
    is( add(0xAB, 0xAB), 0,   'add: 0xAB+0xAB=0' );

    # Commutativity
    is( add(0x12, 0x34), add(0x34, 0x12), 'add: commutative' );

    # Associativity: (a+b)+c = a+(b+c)
    is( add(add(0x12, 0x34), 0x56), add(0x12, add(0x34, 0x56)),
        'add: associative' );
};

subtest 'subtract' => sub {
    # subtract = add in GF(2^8)
    is( subtract(0x53, 0xCA), add(0x53, 0xCA), 'subtract = add' );
    is( subtract(0xFF, 0xFF), 0, 'subtract: a-a=0' );
    is( subtract(1, 1), 0, 'subtract: 1-1=0' );
    is( subtract(5, 3), add(5, 3), 'subtract: 5-3 = 5 XOR 3' );
};

# ============================================================================
# multiply
# ============================================================================
# Multiplication uses the LOG/ALOG tables.

subtest 'multiply' => sub {
    # Multiplicative identity: a * 1 = a
    is( multiply(0x53, 1), 0x53, 'multiply: a*1=a' );
    is( multiply(1, 0xFF), 0xFF, 'multiply: 1*a=a' );

    # Zero: a * 0 = 0
    is( multiply(0x53, 0), 0, 'multiply: a*0=0' );
    is( multiply(0, 0xFF), 0, 'multiply: 0*a=0' );
    is( multiply(0, 0),    0, 'multiply: 0*0=0' );

    # Commutativity
    is( multiply(0x53, 0xCA), multiply(0xCA, 0x53), 'multiply: commutative' );

    # Known inverse pair under primitive polynomial 0x11D:
    # 0x53 * 0x8C = 0x01 in GF(2^8) with poly 0x11D
    # (Verified by exhaustive computation from the ALOG table)
    is( multiply(0x53, 0x8C), 1, 'multiply: 0x53 * 0x8C = 1 (inverses, poly 0x11D)' );

    # Distributivity: a*(b+c) = a*b + a*c
    my $a = 0x12; my $b = 0x34; my $c = 0x56;
    is( multiply($a, add($b, $c)),
        add(multiply($a,$b), multiply($a,$c)),
        'multiply: distributive over add' );

    # generator: 2^8 mod p(x) should equal lower bits of 0x11D (= 0x1D = 29)
    # 2^1=2, 2^2=4, ..., 2^8 wraps
    is( multiply(2, multiply(2, multiply(2, multiply(2,
        multiply(2, multiply(2, multiply(2, 2))))))),
        $CodingAdventures::GF256::ALOG[8],
        'multiply: 2^8 = ALOG[8]' );
};

# ============================================================================
# divide
# ============================================================================

subtest 'divide' => sub {
    # a / 1 = a
    is( divide(0x53, 1), 0x53, 'divide: a/1=a' );

    # a / a = 1 for a != 0
    is( divide(0x53, 0x53), 1, 'divide: a/a=1' );
    is( divide(0xFF, 0xFF), 1, 'divide: 0xFF/0xFF=1' );

    # 0 / a = 0
    is( divide(0, 0x53), 0, 'divide: 0/a=0' );

    # Inverse of divide: (a/b)*b = a
    my $a = 0x53; my $b = 0xCA;
    is( multiply(divide($a, $b), $b), $a, 'divide: (a/b)*b=a' );

    # Division by zero dies
    ok( dies { divide(1, 0) },    'divide: by zero dies' );
    ok( dies { divide(0xFF, 0) }, 'divide: 0xFF/0 dies' );

    # Known inverse pair under poly 0x11D: inverse(0x53) = 0x8C
    is( divide(1, 0x53), 0x8C, 'divide: 1/0x53 = inverse = 0x8C (poly 0x11D)' );
};

# ============================================================================
# power
# ============================================================================

subtest 'power' => sub {
    # a^0 = 1 for all a (including 0^0 = 1 by convention)
    is( power(0x53, 0), 1,    'power: a^0=1' );
    is( power(0,    0), 1,    'power: 0^0=1 by convention' );
    is( power(1,    0), 1,    'power: 1^0=1' );

    # 0^k = 0 for k > 0
    is( power(0, 1),  0, 'power: 0^1=0' );
    is( power(0, 10), 0, 'power: 0^10=0' );

    # 1^k = 1 for all k
    is( power(1, 255), 1, 'power: 1^255=1' );

    # Generator: g^255 = 1 (order of multiplicative group)
    is( power(2, 255), 1, 'power: 2^255=1 (generator order)' );

    # Matches ALOG table
    is( power(2, 1),   $CodingAdventures::GF256::ALOG[1],  'power: 2^1 = ALOG[1]' );
    is( power(2, 10),  $CodingAdventures::GF256::ALOG[10], 'power: 2^10 = ALOG[10]' );
    is( power(2, 100), $CodingAdventures::GF256::ALOG[100],'power: 2^100 = ALOG[100]' );

    # Power law: a^(m+n) = a^m * a^n
    my $base = 0x03;
    is( power($base, 10),
        multiply(power($base, 6), power($base, 4)),
        'power: a^(m+n) = a^m * a^n' );
};

# ============================================================================
# inverse
# ============================================================================

subtest 'inverse' => sub {
    # a * inverse(a) = 1
    for my $a (1, 2, 3, 0x53, 0xCA, 0xFF) {
        is( multiply($a, inverse($a)), 1, "inverse: $a * inverse($a) = 1" );
    }

    # inverse of 1 = 1
    is( inverse(1), 1, 'inverse: inverse(1) = 1' );

    # inverse is its own inverse: inverse(inverse(a)) = a
    is( inverse(inverse(0x53)), 0x53, 'inverse: double inverse returns original' );

    # inverse(0) dies
    ok( dies { inverse(0) }, 'inverse: inverse(0) dies' );

    # Matches divide(1, a)
    is( inverse(0x53), divide(1, 0x53), 'inverse: matches divide(1,a)' );
};

# ============================================================================
# Field axiom checks
# ============================================================================
# A quick spot-check that GF(2^8) satisfies field axioms for a small set of
# representative values.

subtest 'field axioms' => sub {
    my @vals = (1, 2, 3, 0x0F, 0x53, 0xCA, 0xFF);

    for my $a (@vals) {
        # Additive identity: a + 0 = a
        is( add($a, 0), $a, "additive identity: $a + 0 = $a" );

        # Multiplicative identity: a * 1 = a
        is( multiply($a, 1), $a, "mult identity: $a * 1 = $a" );

        # Additive inverse: a + a = 0
        is( add($a, $a), 0, "additive inverse: $a + $a = 0" );

        # Multiplicative inverse: a * a^-1 = 1
        is( multiply($a, inverse($a)), 1, "mult inverse: $a * a^-1 = 1" );
    }
};

done_testing;
