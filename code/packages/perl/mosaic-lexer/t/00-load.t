use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::MosaicLexer; 1 }, 'CodingAdventures::MosaicLexer loads' );

# Verify the module exports a version number.
ok(CodingAdventures::MosaicLexer->VERSION, 'has a VERSION');

done_testing;
