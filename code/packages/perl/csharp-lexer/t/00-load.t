use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::CSharpLexer; 1 },
    'CodingAdventures::CSharpLexer loads' );

ok( CodingAdventures::CSharpLexer->VERSION, 'has a VERSION' );

done_testing;
