use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::MosaicAnalyzer; 1 }, 'CodingAdventures::MosaicAnalyzer loads' );

# Verify the module exports a version number.
ok(CodingAdventures::MosaicAnalyzer->VERSION, 'has a VERSION');

done_testing;
