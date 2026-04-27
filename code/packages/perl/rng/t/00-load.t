use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Rng; 1 }, 'CodingAdventures::Rng loads' );

# Verify the module exports a version number.
ok(CodingAdventures::Rng->VERSION, 'has a VERSION');

done_testing;
