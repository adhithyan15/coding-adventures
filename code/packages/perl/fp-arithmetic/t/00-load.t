use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::FpArithmetic; 1 }, 'CodingAdventures::FpArithmetic loads' );

# Verify the module exports a version number.
ok(CodingAdventures::FpArithmetic->VERSION, 'has a VERSION');

done_testing;
