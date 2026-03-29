use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::TypescriptParser; 1 },
    'CodingAdventures::TypescriptParser loads' );

ok( eval { require CodingAdventures::TypescriptParser::ASTNode; 1 },
    'CodingAdventures::TypescriptParser::ASTNode loads' );

ok( CodingAdventures::TypescriptParser->VERSION, 'TypescriptParser has a VERSION' );
ok( CodingAdventures::TypescriptParser::ASTNode->VERSION, 'ASTNode has a VERSION' );

done_testing;
