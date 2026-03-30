use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::LatticeLexer; 1 },
    'CodingAdventures::LatticeLexer loads' );

ok( CodingAdventures::LatticeLexer->VERSION, 'has a VERSION' );

done_testing;
