use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::SqlLexer; 1 },
    'CodingAdventures::SqlLexer loads' );

ok( CodingAdventures::SqlLexer->VERSION, 'has a VERSION' );

done_testing;
