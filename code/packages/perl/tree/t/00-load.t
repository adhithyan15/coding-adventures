use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Tree; 1 }, 'CodingAdventures::Tree loads' );

# Verify the module exports a version number.
ok(CodingAdventures::Tree->VERSION, 'has a VERSION');

done_testing;
