use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Clock; 1 }, 'CodingAdventures::Clock loads' );

# Verify the module exports a version number.
ok(CodingAdventures::Clock->VERSION, 'has a VERSION');

done_testing;
