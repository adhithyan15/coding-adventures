use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Rng;

# ============================================================================
# LCG — Linear Congruential Generator tests
#
# Reference values for seed=1 (from Go reference implementation):
#   next_u32 call 1: 1817669548
#   next_u32 call 2: 2187888307
#   next_u32 call 3: 2784682393
# ============================================================================

# --- Construction -----------------------------------------------------------

ok( CodingAdventures::Rng::LCG->new(1),  'LCG->new(1) constructs without error' );
ok( CodingAdventures::Rng::LCG->new(0),  'LCG->new(0) is valid' );
ok( CodingAdventures::Rng::LCG->new(42), 'LCG->new(42) constructs without error' );

# --- Reference values -------------------------------------------------------

{
    my $g = CodingAdventures::Rng::LCG->new(1);
    is( $g->next_u32(), 1817669548, 'LCG seed=1: first output matches reference' );
}

{
    my $g = CodingAdventures::Rng::LCG->new(1);
    $g->next_u32();
    is( $g->next_u32(), 2187888307, 'LCG seed=1: second output matches reference' );
}

{
    my $g = CodingAdventures::Rng::LCG->new(1);
    $g->next_u32();
    $g->next_u32();
    is( $g->next_u32(), 2784682393, 'LCG seed=1: third output matches reference' );
}

# --- Determinism ------------------------------------------------------------

{
    my $g1 = CodingAdventures::Rng::LCG->new(99);
    my $g2 = CodingAdventures::Rng::LCG->new(99);
    my $same = 1;
    for (1..10) {
        $same = 0 if $g1->next_u32() != $g2->next_u32();
    }
    ok( $same, 'LCG: same seed produces identical sequences' );
}

{
    my $g1 = CodingAdventures::Rng::LCG->new(1);
    my $g2 = CodingAdventures::Rng::LCG->new(2);
    isnt( $g1->next_u32(), $g2->next_u32(), 'LCG: different seeds produce different first values' );
}

# --- Range checks -----------------------------------------------------------

{
    my $g = CodingAdventures::Rng::LCG->new(7);
    my $ok = 1;
    for (1..20) {
        my $v = $g->next_u32();
        $ok = 0 unless $v >= 0 && $v <= 0xFFFFFFFF;
    }
    ok( $ok, 'LCG: next_u32 always in [0, 2^32-1]' );
}

{
    my $g = CodingAdventures::Rng::LCG->new(123);
    my $ok = 1;
    for (1..20) {
        my $f = $g->next_float();
        $ok = 0 unless $f >= 0.0 && $f < 1.0;
    }
    ok( $ok, 'LCG: next_float always in [0.0, 1.0)' );
}

# --- next_int_in_range -------------------------------------------------------

{
    my $g = CodingAdventures::Rng::LCG->new(5);
    my $ok = 1;
    for (1..50) {
        my $v = $g->next_int_in_range(1, 6);
        $ok = 0 unless $v >= 1 && $v <= 6;
    }
    ok( $ok, 'LCG: next_int_in_range(1,6) always in [1,6]' );
}

{
    my $g = CodingAdventures::Rng::LCG->new(0);
    my %seen;
    for (1..200) {
        $seen{ $g->next_int_in_range(0, 4) }++;
    }
    my $covered = scalar keys %seen == 5;
    ok( $covered, 'LCG: next_int_in_range covers all values 0..4 over 200 draws' );
}

{
    my $g = CodingAdventures::Rng::LCG->new(3);
    my $ok = 1;
    for (1..10) {
        $ok = 0 if $g->next_int_in_range(7, 7) != 7;
    }
    ok( $ok, 'LCG: next_int_in_range(7,7) always returns 7' );
}

# --- next_u64 ----------------------------------------------------------------

{
    my $g1 = CodingAdventures::Rng::LCG->new(99);
    my $g2 = CodingAdventures::Rng::LCG->new(99);
    is( $g1->next_u64()->bstr(), $g2->next_u64()->bstr(), 'LCG: next_u64 is deterministic' );
}

done_testing;
