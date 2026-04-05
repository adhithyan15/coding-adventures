use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::EcmascriptES1Lexer; 1 },
    'CodingAdventures::EcmascriptES1Lexer loads' );

ok( CodingAdventures::EcmascriptES1Lexer->VERSION, 'has a VERSION' );

done_testing;
