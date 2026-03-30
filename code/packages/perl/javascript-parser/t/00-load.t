use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::JavascriptParser; 1 },
    'CodingAdventures::JavascriptParser loads' );

ok( eval { require CodingAdventures::JavascriptParser::ASTNode; 1 },
    'CodingAdventures::JavascriptParser::ASTNode loads' );

ok( CodingAdventures::JavascriptParser->VERSION, 'JavascriptParser has a VERSION' );
ok( CodingAdventures::JavascriptParser::ASTNode->VERSION, 'ASTNode has a VERSION' );

done_testing;
