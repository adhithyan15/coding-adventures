use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::ImmutableList; 1 }, 'CodingAdventures::ImmutableList loads' );

# Verify the module exports a version number.
ok(CodingAdventures::ImmutableList->VERSION, 'has a VERSION');

done_testing;
