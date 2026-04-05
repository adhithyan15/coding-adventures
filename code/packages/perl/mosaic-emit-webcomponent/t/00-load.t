use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::MosaicEmitWebcomponent; 1 }, 'CodingAdventures::MosaicEmitWebcomponent loads' );

# Verify the module exports a version number.
ok(CodingAdventures::MosaicEmitWebcomponent->VERSION, 'has a VERSION');

done_testing;
