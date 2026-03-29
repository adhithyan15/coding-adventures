use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::CommonmarkParser; 1 }, 'CodingAdventures::CommonmarkParser loads' );

# Verify the module exports a version number.
ok(CodingAdventures::CommonmarkParser->VERSION, 'has a VERSION');

done_testing;
