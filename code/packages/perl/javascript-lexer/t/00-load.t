use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::JavascriptLexer; 1 },
    'CodingAdventures::JavascriptLexer loads' );

ok( CodingAdventures::JavascriptLexer->VERSION, 'has a VERSION' );

done_testing;
