use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::CsvParser; 1 }, 'CodingAdventures::CsvParser loads' );

# Verify the module exports a version number.
ok(CodingAdventures::CsvParser->VERSION, 'has a VERSION');

done_testing;
