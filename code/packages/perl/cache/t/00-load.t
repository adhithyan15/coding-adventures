use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::Cache; 1 }, 'CodingAdventures::Cache loads' );

# Verify the module exports a version number.
ok(CodingAdventures::Cache->VERSION, 'has a VERSION');

done_testing;
