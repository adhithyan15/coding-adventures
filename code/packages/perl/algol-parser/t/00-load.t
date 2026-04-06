use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::AlgolParser; 1 },
    'CodingAdventures::AlgolParser loads' );

ok( CodingAdventures::AlgolParser->VERSION, 'has a VERSION' );

done_testing;
