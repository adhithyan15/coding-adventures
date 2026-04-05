use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::EcmascriptES5Lexer; 1 },
    'CodingAdventures::EcmascriptES5Lexer loads' );

ok( CodingAdventures::EcmascriptES5Lexer->VERSION, 'has a VERSION' );

done_testing;
