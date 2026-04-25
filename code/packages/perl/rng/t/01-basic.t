use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Rng;

# Basic smoke tests: all three generator classes are loadable and constructable.

ok( CodingAdventures::Rng::LCG->new(1),        'LCG class is usable' );
ok( CodingAdventures::Rng::Xorshift64->new(1), 'Xorshift64 class is usable' );
ok( CodingAdventures::Rng::PCG32->new(1),      'PCG32 class is usable' );

# Each class implements the required four-method API.
{
    for my $class (
        'CodingAdventures::Rng::LCG',
        'CodingAdventures::Rng::Xorshift64',
        'CodingAdventures::Rng::PCG32',
    ) {
        my $g = $class->new(1);
        ok( $g->can('next_u32'),          "$class implements next_u32" );
        ok( $g->can('next_u64'),          "$class implements next_u64" );
        ok( $g->can('next_float'),        "$class implements next_float" );
        ok( $g->can('next_int_in_range'), "$class implements next_int_in_range" );
    }
}

done_testing;
