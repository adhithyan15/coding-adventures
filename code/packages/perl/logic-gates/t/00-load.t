use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::LogicGates; 1 }, 'CodingAdventures::LogicGates loads' );

# Verify the module exports a version number.
ok(CodingAdventures::LogicGates->VERSION, 'has a VERSION');

done_testing;
