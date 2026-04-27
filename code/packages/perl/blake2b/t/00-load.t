use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Blake2b; 1 }, 'CodingAdventures::Blake2b loads' );

ok(CodingAdventures::Blake2b->VERSION, 'has a VERSION');

done_testing;
