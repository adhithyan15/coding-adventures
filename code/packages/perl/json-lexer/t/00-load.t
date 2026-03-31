use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::JsonLexer; 1 },
    'CodingAdventures::JsonLexer loads' );

ok( CodingAdventures::JsonLexer->VERSION, 'has a VERSION' );

done_testing;
