use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::HttpCore; 1 }, 'CodingAdventures::HttpCore loads' );

# Verify the module exports a version number.
ok(CodingAdventures::HttpCore->VERSION, 'has a VERSION');

done_testing;
