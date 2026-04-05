use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::EcmascriptES3Lexer; 1 },
    'CodingAdventures::EcmascriptES3Lexer loads' );

ok( CodingAdventures::EcmascriptES3Lexer->VERSION, 'has a VERSION' );

done_testing;
