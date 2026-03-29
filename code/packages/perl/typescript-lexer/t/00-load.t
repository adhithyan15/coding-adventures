use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::TypescriptLexer; 1 },
    'CodingAdventures::TypescriptLexer loads' );

ok( CodingAdventures::TypescriptLexer->VERSION, 'has a VERSION' );

done_testing;
