use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::DirectedGraph; 1 }, 'CodingAdventures::DirectedGraph loads' );

# Verify the module exports a version number.
ok(CodingAdventures::DirectedGraph->VERSION, 'has a VERSION');

done_testing;
