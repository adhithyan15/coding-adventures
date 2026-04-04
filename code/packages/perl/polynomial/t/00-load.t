use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Polynomial; 1 }, 'CodingAdventures::Polynomial loads' );

ok( CodingAdventures::Polynomial->VERSION, 'has a VERSION' );

done_testing;
