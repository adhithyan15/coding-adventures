use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Transistors; 1 }, 'CodingAdventures::Transistors loads' );

# Verify the module exports a version number.
ok(CodingAdventures::Transistors->VERSION, 'has a VERSION');

done_testing;
