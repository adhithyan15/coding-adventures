use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::BlockRam; 1 }, 'CodingAdventures::BlockRam loads' );

# Verify the module exports a version number.
ok(CodingAdventures::BlockRam->VERSION, 'has a VERSION');

done_testing;
