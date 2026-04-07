use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::DartmouthBasicLexer; 1 },
    'CodingAdventures::DartmouthBasicLexer loads' );

ok( CodingAdventures::DartmouthBasicLexer->VERSION, 'has a VERSION' );

done_testing;
