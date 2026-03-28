use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::DrawInstructions; 1 }, 'CodingAdventures::DrawInstructions loads' );

# Verify the module exports a version number.
ok(CodingAdventures::DrawInstructions->VERSION, 'has a VERSION');

done_testing;
