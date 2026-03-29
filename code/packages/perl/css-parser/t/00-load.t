use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::CssParser; 1 },
    'CodingAdventures::CssParser loads' );

ok( eval { require CodingAdventures::CssParser::ASTNode; 1 },
    'CodingAdventures::CssParser::ASTNode loads' );

ok( CodingAdventures::CssParser->VERSION, 'CssParser has a VERSION' );
ok( CodingAdventures::CssParser::ASTNode->VERSION, 'ASTNode has a VERSION' );

done_testing;
