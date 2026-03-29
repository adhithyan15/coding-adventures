use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::ProgressBar; 1 }, 'CodingAdventures::ProgressBar loads' );

# Verify the module exports a version number.
ok(CodingAdventures::ProgressBar->VERSION, 'has a VERSION');

done_testing;
