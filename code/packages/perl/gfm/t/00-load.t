use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Gfm; 1 }, 'CodingAdventures::Gfm loads' );

# Verify the module exports a version number.
ok(CodingAdventures::Gfm->VERSION, 'has a VERSION');

done_testing;
