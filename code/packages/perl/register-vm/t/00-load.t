use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::RegisterVM; 1 }, 'CodingAdventures::RegisterVM loads' );

ok( CodingAdventures::RegisterVM->VERSION, 'has a VERSION' );

done_testing;
