use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::GfmParser; 1 }, 'CodingAdventures::GfmParser loads' );

# Verify the module exports a version number.
ok(CodingAdventures::GfmParser->VERSION, 'has a VERSION');

done_testing;
