use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::ExcelLexer; 1 },
    'CodingAdventures::ExcelLexer loads' );

ok( CodingAdventures::ExcelLexer->VERSION, 'has a VERSION' );

done_testing;
