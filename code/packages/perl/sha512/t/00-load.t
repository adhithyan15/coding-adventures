use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Sha512; 1 }, 'CodingAdventures::Sha512 loads' );

# Verify the module exports a version number.
ok(CodingAdventures::Sha512->VERSION, 'has a VERSION');

done_testing;
