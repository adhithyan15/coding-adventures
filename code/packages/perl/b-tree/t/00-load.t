use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::BTree; 1 }, 'CodingAdventures::BTree loads' );
ok( CodingAdventures::BTree->VERSION, 'has a VERSION' );

done_testing;
