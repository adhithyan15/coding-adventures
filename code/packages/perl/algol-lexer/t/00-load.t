use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::AlgolLexer; 1 },
    'CodingAdventures::AlgolLexer loads' );

ok( CodingAdventures::AlgolLexer->VERSION, 'has a VERSION' );

done_testing;
