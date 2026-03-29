use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::DocumentAst; 1 }, 'CodingAdventures::DocumentAst loads' );

# Verify the module exports a version number.
ok(CodingAdventures::DocumentAst->VERSION, 'has a VERSION');

done_testing;
