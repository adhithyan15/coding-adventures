use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Rng;

# ============================================================================
# PCG32 tests
#
# Reference values for seed=1 (from Go reference implementation):
#   next_u32 call 1: 1412771199
#   next_u32 call 2: 1791099446
#   next_u32 call 3: 124312908
# ============================================================================

# --- Construction -----------------------------------------------------------

ok( CodingAdventures::Rng::PCG32->new(1),  'PCG32->new(1) constructs without error' );
ok( CodingAdventures::Rng::PCG32->new(0),  'PCG32->new(0) is valid' );
ok( CodingAdventures::Rng::PCG32->new(42), 'PCG32->new(42) constructs without error' );

# --- Reference values -------------------------------------------------------

{
    my $g = CodingAdventures::Rng::PCG32->new(1);
    is( $g->next_u32(), 1412771199, 'PCG32 seed=1: first output matches reference' );
}

{
    my $g = CodingAdventures::Rng::PCG32->new(1);
    $g->next_u32();
    is( $g->next_u32(), 1791099446, 'PCG32 seed=1: second output matches reference' );
}

{
    my $g = CodingAdventures::Rng::PCG32->new(1);
    $g->next_u32();
    $g->next_u32();
    is( $g->next_u32(), 124312908, 'PCG32 seed=1: third output matches reference' );
}

# --- Determinism ------------------------------------------------------------

{
    my $g1 = CodingAdventures::Rng::PCG32->new(12345);
    my $g2 = CodingAdventures::Rng::PCG32->new(12345);
    my $same = 1;
    for (1..10) {
        $same = 0 if $g1->next_u32() != $g2->next_u32();
    }
    ok( $same, 'PCG32: same seed produces identical sequences' );
}

{
    my $g1 = CodingAdventures::Rng::PCG32->new(1);
    my $g2 = CodingAdventures::Rng::PCG32->new(2);
    isnt( $g1->next_u32(), $g2->next_u32(), 'PCG32: different seeds produce different first values' );
}

# PCG32 uses a different output permutation than plain LCG
{
    my $lcg = CodingAdventures::Rng::LCG->new(1);
    my $pcg = CodingAdventures::Rng::PCG32->new(1);
    isnt( $lcg->next_u32(), $pcg->next_u32(), 'PCG32 and LCG produce different outputs for same seed' );
}

# --- Range checks -----------------------------------------------------------

{
    my $g = CodingAdventures::Rng::PCG32->new(7);
    my $ok = 1;
    for (1..20) {
        my $v = $g->next_u32();
        $ok = 0 unless $v >= 0 && $v <= 0xFFFFFFFF;
    }
    ok( $ok, 'PCG32: next_u32 always in [0, 2^32-1]' );
}

{
    my $g = CodingAdventures::Rng::PCG32->new(123);
    my $ok = 1;
    for (1..20) {
        my $f = $g->next_float();
        $ok = 0 unless $f >= 0.0 && $f < 1.0;
    }
    ok( $ok, 'PCG32: next_float always in [0.0, 1.0)' );
}

# --- next_int_in_range -------------------------------------------------------

{
    my $g = CodingAdventures::Rng::PCG32->new(5);
    my $ok = 1;
    for (1..50) {
        my $v = $g->next_int_in_range(1, 6);
        $ok = 0 unless $v >= 1 && $v <= 6;
    }
    ok( $ok, 'PCG32: next_int_in_range(1,6) always in [1,6]' );
}

{
    my $g = CodingAdventures::Rng::PCG32->new(3);
    my %seen;
    for (1..200) {
        $seen{ $g->next_int_in_range(0, 4) }++;
    }
    my $covered = scalar keys %seen == 5;
    ok( $covered, 'PCG32: next_int_in_range covers all values 0..4 over 200 draws' );
}

{
    my $g = CodingAdventures::Rng::PCG32->new(9);
    my $ok = 1;
    for (1..10) {
        $ok = 0 if $g->next_int_in_range(100, 100) != 100;
    }
    ok( $ok, 'PCG32: next_int_in_range(100,100) always returns 100' );
}

# --- next_u64 ----------------------------------------------------------------

{
    my $g1 = CodingAdventures::Rng::PCG32->new(11);
    my $g2 = CodingAdventures::Rng::PCG32->new(11);
    is( $g1->next_u64()->bstr(), $g2->next_u64()->bstr(), 'PCG32: next_u64 is deterministic' );
}

done_testing;
