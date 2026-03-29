use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::GrammarTools; 1 }, 'CodingAdventures::GrammarTools loads' );

# Verify the module exports a version number.
ok(CodingAdventures::GrammarTools->VERSION, 'has a VERSION');

done_testing;
