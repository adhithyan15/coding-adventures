use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::MosaicVm; 1 }, 'CodingAdventures::MosaicVm loads' );

# Verify the module exports a version number.
ok(CodingAdventures::MosaicVm->VERSION, 'has a VERSION');

done_testing;
