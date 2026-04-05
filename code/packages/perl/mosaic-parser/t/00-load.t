use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::MosaicParser; 1 }, 'CodingAdventures::MosaicParser loads' );

# Verify the module exports a version number.
ok(CodingAdventures::MosaicParser->VERSION, 'has a VERSION');

done_testing;
