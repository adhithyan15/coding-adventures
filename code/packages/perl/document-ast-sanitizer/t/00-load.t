use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::DocumentAstSanitizer; 1 }, 'CodingAdventures::DocumentAstSanitizer loads' );

# Verify the module exports a version number.
ok(CodingAdventures::DocumentAstSanitizer->VERSION, 'has a VERSION');

done_testing;
