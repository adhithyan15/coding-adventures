use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::ExcelParser; 1 },
    'CodingAdventures::ExcelParser loads' );

ok( CodingAdventures::ExcelParser->VERSION, 'has a VERSION' );

ok( eval { require CodingAdventures::ExcelParser::ASTNode; 1 },
    'CodingAdventures::ExcelParser::ASTNode loads' );

done_testing;
