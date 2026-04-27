use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Http1; 1 }, 'CodingAdventures::Http1 loads' );

# Verify the module exports a version number.
ok(CodingAdventures::Http1->VERSION, 'has a VERSION');

done_testing;
