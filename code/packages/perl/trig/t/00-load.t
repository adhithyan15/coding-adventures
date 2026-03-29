use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Trig; 1 }, 'CodingAdventures::Trig loads' );

# Verify the module exports a version number.
ok(CodingAdventures::Trig->VERSION, 'has a VERSION');

done_testing;
