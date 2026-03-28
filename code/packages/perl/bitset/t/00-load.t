use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Bitset; 1 }, 'CodingAdventures::Bitset loads' );

# Verify the module exports a version number.
ok(CodingAdventures::Bitset->VERSION, 'has a VERSION');

done_testing;
