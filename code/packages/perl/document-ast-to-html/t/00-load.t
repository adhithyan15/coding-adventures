use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::DocumentAstToHtml; 1 }, 'CodingAdventures::DocumentAstToHtml loads' );

# Verify the module exports a version number.
ok(CodingAdventures::DocumentAstToHtml->VERSION, 'has a VERSION');

done_testing;
