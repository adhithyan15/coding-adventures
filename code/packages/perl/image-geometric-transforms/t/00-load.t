use strict;
use warnings;
use lib '../pixel-container/lib';
use Test2::V0;

ok( eval { require CodingAdventures::ImageGeometricTransforms; 1 }, 'CodingAdventures::ImageGeometricTransforms loads' );

# Verify the module exports a version number.
ok(CodingAdventures::ImageGeometricTransforms->VERSION, 'has a VERSION');

done_testing;
