use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::MosaicEmitReact; 1 }, 'CodingAdventures::MosaicEmitReact loads' );

# Verify the module exports a version number.
ok(CodingAdventures::MosaicEmitReact->VERSION, 'has a VERSION');

done_testing;
