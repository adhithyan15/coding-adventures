use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Actor; 1 }, 'CodingAdventures::Actor loads' );

# Verify the module exports a version number.
ok(CodingAdventures::Actor->VERSION, 'has a VERSION');

done_testing;
