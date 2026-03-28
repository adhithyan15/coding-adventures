use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Sha1; 1 }, 'CodingAdventures::Sha1 loads' );

# Verify the module exports a version number.
ok(CodingAdventures::Sha1->VERSION, 'has a VERSION');

done_testing;
