use strict;
use warnings;
use lib '../pixel-container/lib';
use Test2::V0;

ok( eval { require CodingAdventures::ImagePointOps; 1 }, 'CodingAdventures::ImagePointOps loads' );

# Verify the module exports a version number.
ok(CodingAdventures::ImagePointOps->VERSION, 'has a VERSION');

done_testing;
