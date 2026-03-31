use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::AtbashCipher; 1 }, 'CodingAdventures::AtbashCipher loads' );

# Verify the module exports a version number.
ok(CodingAdventures::AtbashCipher->VERSION, 'has a VERSION');

done_testing;
