use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::CssLexer; 1 },
    'CodingAdventures::CssLexer loads' );

ok( CodingAdventures::CssLexer->VERSION, 'has a VERSION' );

done_testing;
