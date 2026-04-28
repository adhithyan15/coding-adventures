use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Rng;

# ============================================================================
# Xorshift64 tests
#
# Reference values for seed=1 (from Go reference implementation):
#   next_u32 call 1: 1082269761
#   next_u32 call 2: 201397313
#   next_u32 call 3: 1854285353
# ============================================================================

# --- Construction -----------------------------------------------------------

ok( CodingAdventures::Rng::Xorshift64->new(1),  'Xorshift64->new(1) constructs without error' );
ok( CodingAdventures::Rng::Xorshift64->new(42), 'Xorshift64->new(42) constructs without error' );

# --- Seed 0 replaced with 1 -------------------------------------------------

{
    my $g0 = CodingAdventures::Rng::Xorshift64->new(0);
    my $g1 = CodingAdventures::Rng::Xorshift64->new(1);
    is( $g0->next_u32(), $g1->next_u32(), 'Xorshift64: seed 0 is replaced with 1' );
}

# --- Reference values -------------------------------------------------------

{
    my $g = CodingAdventures::Rng::Xorshift64->new(1);
    is( $g->next_u32(), 1082269761, 'Xorshift64 seed=1: first output matches reference' );
}

{
    my $g = CodingAdventures::Rng::Xorshift64->new(1);
    $g->next_u32();
    is( $g->next_u32(), 201397313, 'Xorshift64 seed=1: second output matches reference' );
}

{
    my $g = CodingAdventures::Rng::Xorshift64->new(1);
    $g->next_u32();
    $g->next_u32();
    is( $g->next_u32(), 1854285353, 'Xorshift64 seed=1: third output matches reference' );
}

# --- Determinism ------------------------------------------------------------

{
    my $g1 = CodingAdventures::Rng::Xorshift64->new(55);
    my $g2 = CodingAdventures::Rng::Xorshift64->new(55);
    my $same = 1;
    for (1..10) {
        $same = 0 if $g1->next_u32() != $g2->next_u32();
    }
    ok( $same, 'Xorshift64: same seed produces identical sequences' );
}

# --- Range checks -----------------------------------------------------------

{
    my $g = CodingAdventures::Rng::Xorshift64->new(7);
    my $ok = 1;
    for (1..20) {
        my $v = $g->next_u32();
        $ok = 0 unless $v >= 0 && $v <= 0xFFFFFFFF;
    }
    ok( $ok, 'Xorshift64: next_u32 always in [0, 2^32-1]' );
}

{
    my $g = CodingAdventures::Rng::Xorshift64->new(123);
    my $ok = 1;
    for (1..20) {
        my $f = $g->next_float();
        $ok = 0 unless $f >= 0.0 && $f < 1.0;
    }
    ok( $ok, 'Xorshift64: next_float always in [0.0, 1.0)' );
}

# --- next_int_in_range -------------------------------------------------------

{
    my $g = CodingAdventures::Rng::Xorshift64->new(5);
    my $ok = 1;
    for (1..50) {
        my $v = $g->next_int_in_range(1, 6);
        $ok = 0 unless $v >= 1 && $v <= 6;
    }
    ok( $ok, 'Xorshift64: next_int_in_range(1,6) always in [1,6]' );
}

{
    my $g = CodingAdventures::Rng::Xorshift64->new(2);
    my %seen;
    for (1..200) {
        $seen{ $g->next_int_in_range(0, 4) }++;
    }
    my $covered = scalar keys %seen == 5;
    ok( $covered, 'Xorshift64: next_int_in_range covers all values 0..4 over 200 draws' );
}

{
    my $g = CodingAdventures::Rng::Xorshift64->new(3);
    my $ok = 1;
    for (1..10) {
        $ok = 0 if $g->next_int_in_range(42, 42) != 42;
    }
    ok( $ok, 'Xorshift64: next_int_in_range(42,42) always returns 42' );
}

# --- next_u64 ----------------------------------------------------------------

{
    my $g1 = CodingAdventures::Rng::Xorshift64->new(77);
    my $g2 = CodingAdventures::Rng::Xorshift64->new(77);
    is( $g1->next_u64()->bstr(), $g2->next_u64()->bstr(), 'Xorshift64: next_u64 is deterministic' );
}

done_testing;
