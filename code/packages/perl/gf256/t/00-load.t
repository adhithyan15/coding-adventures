use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::GF256; 1 }, 'CodingAdventures::GF256 loads' );

ok( CodingAdventures::GF256->VERSION, 'has a VERSION' );

done_testing;
