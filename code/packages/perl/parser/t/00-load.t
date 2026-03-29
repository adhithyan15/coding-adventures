use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Parser; 1 }, 'CodingAdventures::Parser loads' );

# Verify the module exports a version number.
ok(CodingAdventures::Parser->VERSION, 'has a VERSION');

done_testing;
