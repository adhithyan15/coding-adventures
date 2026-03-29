use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Arithmetic; 1 }, 'CodingAdventures::Arithmetic loads' );

# Verify the module exports a version number.
ok(CodingAdventures::Arithmetic->VERSION, 'has a VERSION');

done_testing;
