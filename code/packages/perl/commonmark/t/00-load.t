use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Commonmark; 1 }, 'CodingAdventures::Commonmark loads' );

# Verify the module exports a version number.
ok(CodingAdventures::Commonmark->VERSION, 'has a VERSION');

done_testing;
