use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Display; 1 }, 'CodingAdventures::Display loads' );

# Verify the module exports a version number.
ok(CodingAdventures::Display->VERSION, 'has a VERSION');

done_testing;
